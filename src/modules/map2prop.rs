use std::path::{Path, PathBuf};

use glam::DVec3;

use eyre::eyre;
use map::Map;
use qc::Qc;
use smd::Smd;
use wad::Wad;

use rayon::prelude::*;

use crate::utils::{
    map_stuffs::{map_to_triangulated_smd_3_points, textures_used_in_map},
    run_bin::run_studiomdl,
    smd_stuffs::{add_bitmap_extension_to_texture, find_centroid, move_by},
    wad_stuffs::{export_texture, SimpleWad},
};

pub static GCHIMP_MAP2PROP_ENTITY_NAME: &str = "gchimp_map2prop";

#[derive(Clone, Copy, Debug)]
pub enum Collision {
    /// No collision. This means the brush will be removed from the map.
    None,
    /// Collision is the same as the original brush. The original brush will have CLIP texture.
    Precise,
    /// The smallest rectangular prism containing the whole brush.
    Box,
}

#[derive(Clone, Debug)]
pub struct MarkedEntity {
    /// Name of the output model
    name: Option<String>,
    /// Collision of the marked entity
    collision: Collision,
    /// Offsets the center of the model. This does not offset the model itself. Only the center point.
    ///
    /// The center would be the centroid, average of all vertices. Not the centroid of the bounding box.
    centroid_offset: DVec3,
}

impl Default for MarkedEntity {
    fn default() -> Self {
        Self {
            name: None,
            collision: Collision::Precise,
            centroid_offset: Default::default(),
        }
    }
}

impl MarkedEntity {
    pub fn collision_none(&mut self) -> &mut Self {
        self.collision = Collision::None;
        self
    }

    pub fn collision_precise(&mut self) -> &mut Self {
        self.collision = Collision::Precise;
        self
    }

    pub fn collision_box(&mut self) -> &mut Self {
        self.collision = Collision::Box;
        self
    }

    pub fn centroid_offset(&mut self, offset: DVec3) -> &mut Self {
        self.centroid_offset = offset;
        self
    }

    pub fn model_name(&mut self, v: &str) -> &mut Self {
        self.name = v.to_owned().into();
        self
    }
}

#[derive(Debug)]
pub struct Map2PropOptions {
    /// If input entity has "wad" key then we get texture from there.
    auto_pickup_wad: bool,
    /// Exports necessary texture for model compilation.
    export_texture: bool,
    /// The entity is moved to the origin so it's overall boxed shape is balanced.
    ///
    /// ORIGIN brush will overwrite this option.
    move_to_origin: bool,
    studiomdl: Option<PathBuf>,
    #[cfg(target_os = "linux")]
    wineprefix: Option<String>,
    /// Only converts [`GCHIMP_MAP2PROP_ENTITY_NAME`] entity
    ///
    /// Not only this will creates a new model but potentially a new .map file
    marked_entity: Option<MarkedEntity>,
}

impl Default for Map2PropOptions {
    fn default() -> Self {
        Self {
            auto_pickup_wad: true,
            export_texture: true,
            marked_entity: None,
            move_to_origin: true,
            studiomdl: None,
            #[cfg(target_os = "linux")]
            wineprefix: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct Map2Prop {
    options: Map2PropOptions,
    map: Option<PathBuf>,
    wads: Vec<PathBuf>,
}

impl Map2Prop {
    pub fn auto_pickup_wad(&mut self, v: bool) -> &mut Self {
        self.options.auto_pickup_wad = v;
        self
    }

    pub fn export_texture(&mut self, v: bool) -> &mut Self {
        self.options.export_texture = v;
        self
    }

    pub fn move_to_origin(&mut self, v: bool) -> &mut Self {
        self.options.move_to_origin = v;
        self
    }

    pub fn add_wad(&mut self, v: &Path) -> &mut Self {
        self.wads.push(v.to_path_buf());
        self
    }

    pub fn marked_entity(&mut self, f: impl Fn(MarkedEntity) -> MarkedEntity) -> &mut Self {
        self.options.marked_entity = Some(f(MarkedEntity::default()));
        self
    }

    pub fn studiomdl(&mut self, v: &Path) -> &mut Self {
        self.options.studiomdl = v.to_path_buf().into();
        self
    }

    #[cfg(target_os = "linux")]
    pub fn wineprefix(&mut self, v: &str) -> &mut Self {
        self.options.wineprefix = v.to_string().into();
        self
    }

    pub fn map_file(&mut self, v: &str) -> &mut Self {
        self.map = PathBuf::from(v).into();
        self
    }

    pub fn work(&mut self) -> eyre::Result<()> {
        if self.options.studiomdl.is_none() {
            return Err(eyre!("No studiomdl.exe supplied."));
        }

        #[cfg(target_os = "linux")]
        if self.options.wineprefix.is_none() {
            return Err(eyre!("No WINEPREFIX supplied."));
        }

        // TODO convert from pasted entity instead of whole map
        let map = if let Some(path) = &self.map {
            Map::from_file(path).ok()
        } else {
            None
        };

        // now we are collecting wad files
        let valid_autopickup_wad = self.options.auto_pickup_wad
            && (map.is_none()
                || map.as_ref().unwrap().entities[0] // always entity 0
                    .attributes
                    .get("wad")
                    .is_some_and(|paths| !paths.is_empty()));

        if self.wads.is_empty() && (!valid_autopickup_wad) {
            return Err(eyre!("No WAD files or MAP supplied."));
        }

        let wads_res = if !self.wads.is_empty() {
            self.wads
                .iter()
                .map(Wad::from_file)
                .collect::<Vec<eyre::Result<Wad>>>()
        } else if valid_autopickup_wad {
            map.as_ref().unwrap().entities[0]
                .attributes
                .get("wad")
                .unwrap()
                .split_terminator(";")
                .map(Wad::from_file)
                .collect::<Vec<eyre::Result<Wad>>>()
        } else {
            unreachable!()
        };

        let err = wads_res
            .iter()
            .filter_map(|res| res.as_ref().err())
            .fold(String::new(), |acc, e| acc + e.to_string().as_ref() + "\n");

        if !err.is_empty() {
            return Err(eyre!("{}", err));
        }

        let wads = wads_res
            .iter()
            .filter_map(|res| res.as_ref().ok())
            .collect::<Vec<&Wad>>();

        let simple_wads: SimpleWad = wads.as_slice().into();

        // after having wad files, now we have to check if entity contains wad in the wad list
        if let Some(map) = &map {
            let textures_used_in_map = textures_used_in_map(map);

            // let missing = simple_wads
            //     .iter()
            //     .filter_map(|(key, _)| {
            //         if textures_used_in_map.contains(key) {
            //             None
            //         } else {
            //             Some(key.to_owned())
            //         }
            //     })
            //     .collect::<Vec<String>>();
            let missing = textures_used_in_map
                .iter()
                .filter_map(|tex| {
                    if simple_wads.get(tex).is_some() {
                        None
                    } else {
                        Some(tex.to_owned())
                    }
                })
                .collect::<Vec<String>>();

            if !missing.is_empty() {
                return Err(eyre!("Missing textures: {:?}", missing));
            }

            // ok to export texture now
            if self.options.export_texture {
                if let Some(err) = textures_used_in_map
                    .par_iter()
                    .map(|tex| {
                        export_texture(
                            wads[simple_wads.get(tex).unwrap().wad_file_index()],
                            tex,
                            self.map.as_ref().unwrap().with_file_name(tex),
                        )
                    })
                    .find_any(|res| res.is_err())
                {
                    return err;
                }
            }
        }

        // right now we have exported texture and everything done setup
        // just need to create smd and then qc
        let output_path = if let Some(map) = &self.map {
            map.to_path_buf()
        } else {
            todo!()
        };

        let smd_triangles_res = if let Some(marked_entity) = &self.options.marked_entity {
            todo!()
        } else if let Some(map) = &map {
            map_to_triangulated_smd_3_points(map, &simple_wads)
        } else {
            unreachable!()
        };

        // stupid clippy
        #[allow(clippy::question_mark)]
        if let Err(err) = smd_triangles_res {
            return Err(err);
        }

        let mut new_smd = Smd::new_basic();

        let smd_triangles = smd_triangles_res.unwrap();
        smd_triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        if self.options.move_to_origin {
            let centroid = find_centroid(&new_smd).unwrap();

            move_by(&mut new_smd, -centroid);
        }

        // TODO split smd
        add_bitmap_extension_to_texture(&mut new_smd);
        new_smd.write(output_path.with_extension("smd"))?;

        // idle smd
        Smd::new_basic().write(output_path.with_file_name("idle.smd"))?;

        // now writes qc
        let mut new_qc = Qc::new_basic();

        new_qc.add_model_name(output_path.with_extension("smd").to_str().unwrap());
        new_qc.add_cd(output_path.parent().unwrap().to_str().unwrap());
        new_qc.add_cd_texture(output_path.parent().unwrap().to_str().unwrap());

        new_qc.add_body(
            "studio0",
            output_path.file_stem().unwrap().to_str().unwrap(),
            false,
            None,
        );
        new_qc.add_sequence("idle", "idle", vec![]);

        new_qc.write(output_path.with_extension("qc"))?;

        let handle = run_studiomdl(
            output_path.with_extension("qc").as_path(),
            self.options.studiomdl.as_ref().unwrap(),
            #[cfg(target_os = "linux")]
            self.options.wineprefix.as_ref().unwrap(),
        );

        handle.join().unwrap()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spawn() {
        let mut binding = Map2Prop::default();

        let a = binding.marked_entity(|mut e| {
            e.collision_box()
                .centroid_offset([0.5, 0.5, 0.5].into())
                .clone()
        });

        assert!(a.options.marked_entity.is_some());
        assert_eq!(
            a.options.marked_entity.clone().unwrap().centroid_offset,
            [0.5, 0.5, 0.5].into()
        );
    }

    #[test]
    fn run() {
        let mut binding = Map2Prop::default();
        binding
            .auto_pickup_wad(true)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map_file("/home/khang/gchimp/examples/map2prop/map.map")
            .work()
            .unwrap();
    }

    #[test]
    fn run2() {
        let mut binding = Map2Prop::default();
        binding
            .auto_pickup_wad(true)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map_file("/home/khang/gchimp/examples/map2prop/map2.map")
            .work()
            .unwrap();
    }
}
