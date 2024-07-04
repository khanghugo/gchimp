use std::path::{Path, PathBuf};

use glam::DVec3;

use map::Map;
use qc::Qc;
use smd::{Smd, Triangle};
use wad::Wad;

use rayon::prelude::*;

use crate::{
    err,
    utils::{
        constants::{MAX_GOLDSRC_MODEL_TEXTURE_COUNT, NO_RENDER_TEXTURE, ORIGIN_TEXTURE},
        map_stuffs::{map_to_triangulated_smd_3_points, textures_used_in_map},
        run_bin::run_studiomdl,
        smd_stuffs::{
            add_bitmap_extension_to_texture, find_centroid, find_centroid_from_triangles,
            maybe_split_smd, move_by, with_selected_textures,
        },
        wad_stuffs::{export_texture, SimpleWad},
    },
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
    /// ORIGIN brush will only work if this is enabled.
    move_to_origin: bool,
    /// Ignores "no draw" textures like sky or NULL
    ignore_nodraw: bool,
    studiomdl: Option<PathBuf>,
    #[cfg(target_os = "linux")]
    wineprefix: Option<String>,
    /// Only converts [`GCHIMP_MAP2PROP_ENTITY_NAME`] entity
    ///
    /// Not only this will creates a new model but potentially a new .map file
    marked_entity: Vec<MarkedEntity>,
}

impl Default for Map2PropOptions {
    fn default() -> Self {
        Self {
            auto_pickup_wad: true,
            export_texture: true,
            marked_entity: vec![],
            move_to_origin: true,
            studiomdl: None,
            #[cfg(target_os = "linux")]
            wineprefix: None,
            ignore_nodraw: true,
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

    pub fn add_marked_entity(&mut self, f: impl Fn(MarkedEntity) -> MarkedEntity) -> &mut Self {
        self.options.marked_entity.push(f(MarkedEntity::default()));
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
            return err!("No studiomdl.exe supplied.");
        }

        #[cfg(target_os = "linux")]
        if self.options.wineprefix.is_none() {
            return err!("No WINEPREFIX supplied.");
        }

        // TODO convert from pasted entity instead of whole map
        let map_file = if let Some(path) = &self.map {
            Map::from_file(path).ok()
        } else {
            None
        };

        // now we are collecting wad files
        let valid_autopickup_wad = self.options.auto_pickup_wad
            && (map_file.is_none()
                || map_file.as_ref().unwrap().entities[0] // always entity 0
                    .attributes
                    .get("wad")
                    .is_some_and(|paths| !paths.is_empty()));

        if self.wads.is_empty() && (!valid_autopickup_wad) {
            return err!("No WAD files or MAP supplied.");
        }

        let wads_res = if !self.wads.is_empty() {
            self.wads
                .iter()
                .map(Wad::from_file)
                .collect::<Vec<eyre::Result<Wad>>>()
        } else if valid_autopickup_wad {
            map_file.as_ref().unwrap().entities[0]
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
            return err!("{}", err);
        }

        let wads = wads_res
            .iter()
            .filter_map(|res| res.as_ref().ok())
            .collect::<Vec<&Wad>>();

        let simple_wads: SimpleWad = wads.as_slice().into();

        let textures_used = if let Some(map) = &map_file {
            textures_used_in_map(map)
        } else {
            todo!()
        };

        let textures_missing = textures_used
            .iter()
            .filter_map(|tex| {
                if simple_wads.get(tex).is_some() {
                    None
                } else {
                    Some(tex.to_owned())
                }
            })
            .collect::<Vec<String>>();

        if !textures_missing.is_empty() {
            return err!("Missing textures: {:?}", textures_missing);
        }

        if self.options.export_texture {
            if let Some(err) = textures_used
                .par_iter()
                .filter(|tex| {
                    if self.options.ignore_nodraw {
                        !NO_RENDER_TEXTURE.contains(&tex.as_str())
                    } else {
                        true
                    }
                })
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

        let smd_triangles_res = if !self.options.marked_entity.is_empty() {
            todo!()
        } else if let Some(map) = &map_file {
            map_to_triangulated_smd_3_points(map, &simple_wads)
        } else {
            unreachable!()
        };

        // stupid clippy
        #[allow(clippy::question_mark)]
        if let Err(err) = smd_triangles_res {
            return Err(err);
        }

        // this smd would contain everything
        // latter smd would be derived from this
        let mut main_smd = Smd::new_basic();

        let smd_triangles = smd_triangles_res.unwrap();

        // if no ORIGIN brush given, then the centroid will be the centroid of the brush
        let origin_brush_triangles = smd_triangles
            .iter()
            .filter(|tri| tri.material == ORIGIN_TEXTURE)
            .cloned()
            .collect::<Vec<Triangle>>();

        smd_triangles
            .into_iter()
            .filter(|tri| {
                if self.options.ignore_nodraw {
                    !NO_RENDER_TEXTURE.contains(&tri.material.as_str())
                } else {
                    true
                }
            })
            .for_each(|tri| {
                main_smd.add_triangle(tri);
            });

        let brush_centroid = if origin_brush_triangles.is_empty() {
            find_centroid(&main_smd).unwrap()
        } else {
            find_centroid_from_triangles(&origin_brush_triangles).unwrap()
        };

        if self.options.move_to_origin {
            move_by(&mut main_smd, -brush_centroid);
        }

        // DO NOT ADD EXTENSION HERE, YET
        // it should be the last step
        // because we are still processing over some data
        // add_bitmap_extension_to_texture(&mut main_smd);

        // before splitting smd, we need to check if we want to split model
        let model_count = textures_used.len() / MAX_GOLDSRC_MODEL_TEXTURE_COUNT + 1;
        let texture_used_vec = textures_used.into_iter().collect::<Vec<String>>();

        let output_path = if let Some(map) = &self.map {
            map.to_path_buf()
        } else {
            todo!()
        };

        // the format of the file will follow
        // smd: <output><model index>_<smd_index>
        // mdl/qc: <output><model index>
        // even if there is 1 modela nd 1 smd, too bad

        // idle smd
        // every model uses the same idle smd so that's ok
        Smd::new_basic().write(output_path.with_file_name("idle.smd"))?;

        let smd_and_qc_res = (0..model_count)
            .map(|model_index| {
                let model_name = format!(
                    "{}{}",
                    output_path.file_stem().unwrap().to_str().unwrap(),
                    model_index,
                );

                let current_model_textures = texture_used_vec
                    .chunks(MAX_GOLDSRC_MODEL_TEXTURE_COUNT)
                    .nth(model_index)
                    .unwrap();

                let curr_model_main_smd =
                    with_selected_textures(&main_smd, current_model_textures)?;
                let smds_to_write = maybe_split_smd(&curr_model_main_smd);
                let smd_count = smds_to_write.len();

                let smd_write_res = smds_to_write
                    .into_par_iter()
                    // ~no need to add extension because it is already done~
                    // actually do it here
                    .map(|mut smd| {
                        add_bitmap_extension_to_texture(&mut smd); // fix extension
                        smd
                    })
                    .enumerate()
                    .map(|(smd_index, smd)| {
                        let smd_name = format!("{}_{}.smd", model_name, smd_index);

                        smd.write(output_path.with_file_name(smd_name))?;

                        Ok(())
                    })
                    .filter_map(|res| res.err())
                    .collect::<Vec<eyre::Report>>();

                if !smd_write_res.is_empty() {
                    let cum_err = smd_write_res
                        .into_iter()
                        .fold(String::new(), |acc, e| acc + e.to_string().as_str() + "\n");
                    return err!(cum_err);
                }

                // now writes qc
                let mut new_qc = Qc::new_basic();

                // fix rotation
                new_qc.add_origin(0., 0., 0., Some(270.));

                new_qc.add_model_name(
                    output_path
                        .with_file_name(format!("{}.mdl", model_name))
                        .to_str()
                        .unwrap(),
                );
                new_qc.add_cd(output_path.parent().unwrap().to_str().unwrap());
                new_qc.add_cd_texture(output_path.parent().unwrap().to_str().unwrap());

                for smd_index in 0..smd_count {
                    new_qc.add_body(
                        format!("studio{}", smd_index).as_str(),
                        format!("{}_{}", model_name, smd_index).as_str(),
                        false,
                        None,
                    );
                }

                new_qc.add_sequence("idle", "idle", vec![]);

                let qc_out_path = output_path.with_file_name(format!("{}.qc", model_name));

                new_qc.write(qc_out_path.as_path())?;

                Ok(qc_out_path)
            })
            // what the fuck
            .collect::<Vec<eyre::Result<PathBuf>>>();

        let err_str = smd_and_qc_res
            .iter()
            .filter_map(|res| res.as_ref().err())
            .fold(String::new(), |acc, e| acc + e.to_string().as_str() + "\n");

        if !err_str.is_empty() {
            return err!(err_str);
        }

        smd_and_qc_res.into_par_iter().for_each(|res| {
            let handle = run_studiomdl(
                res.unwrap().as_path(),
                self.options.studiomdl.as_ref().unwrap(),
                #[cfg(target_os = "linux")]
                self.options.wineprefix.as_ref().unwrap(),
            );

            let _ = handle.join().unwrap();
        });

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spawn() {
        let mut binding = Map2Prop::default();

        let a = binding.add_marked_entity(|mut e| {
            e.collision_box()
                .centroid_offset([0.5, 0.5, 0.5].into())
                .clone()
        });

        assert!(!a.options.marked_entity.is_empty());
        assert_eq!(
            a.options.marked_entity[0].centroid_offset,
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

    #[test]
    fn arte_twist() {
        let mut binding = Map2Prop::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map_file("/home/khang/gchimp/examples/map2prop/arte_spin/arte_spin.map")
            .work()
            .unwrap();
    }

    #[test]
    fn sphere() {
        let mut binding = Map2Prop::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map_file("/home/khang/gchimp/examples/map2prop/sphere.map")
            .work()
            .unwrap();
    }

    #[test]
    fn sphere2() {
        let mut binding = Map2Prop::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map_file("/home/khang/gchimp/examples/map2prop/sphere2.map")
            .work()
            .unwrap();
    }
}
