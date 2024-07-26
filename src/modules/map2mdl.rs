use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use map::{Attributes, Entity, Map};
use qc::Qc;
use smd::{Smd, Triangle};
use wad::Wad;

use rayon::{iter::Either, prelude::*};

use crate::{
    err,
    utils::{
        constants::{
            CLIP_TEXTURE, GCHIMP_INFO_ENTITY, MAX_GOLDSRC_MODEL_TEXTURE_COUNT, NO_RENDER_TEXTURE,
            ORIGIN_TEXTURE,
        },
        map_stuffs::{
            brush_from_mins_maxs, check_gchimp_info_entity, entity_to_triangulated_smd,
            map_to_triangulated_smd, textures_used_in_entity, textures_used_in_map,
        },
        run_bin::run_studiomdl,
        smd_stuffs::{
            add_bitmap_extension_to_texture, find_centroid, find_centroid_from_triangles,
            find_mins_maxs, maybe_split_smd, move_by, with_selected_textures,
        },
        wad_stuffs::{export_texture, SimpleWad},
    },
};

pub static GCHIMP_MAP2MDL_ENTITY_NAME: &str = "gchimp_map2mdl";

#[derive(Debug)]
pub struct Map2MdlOptions {
    /// If input entity has "wad" key then we get texture from there.
    pub auto_pickup_wad: bool,
    /// Exports necessary texture for model compilation.
    pub export_texture: bool,
    /// The entity is moved to the origin so it's overall boxed shape is balanced.
    ///
    /// ORIGIN brush will only work if this is enabled.
    pub move_to_origin: bool,
    /// Ignores "no draw" textures like sky or NULL
    pub ignore_nodraw: bool,
    studiomdl: Option<PathBuf>,
    #[cfg(target_os = "linux")]
    wineprefix: Option<String>,
    /// Only converts [`GCHIMP_MAP2MDL_ENTITY_NAME`] entity
    ///
    /// Not only this will creates a new model but potentially a new .map file
    pub marked_entity: bool,
}

impl Default for Map2MdlOptions {
    fn default() -> Self {
        Self {
            auto_pickup_wad: true,
            export_texture: true,
            marked_entity: false,
            move_to_origin: true,
            studiomdl: None,
            #[cfg(target_os = "linux")]
            wineprefix: None,
            ignore_nodraw: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Map2MdlSync {
    stdout: Arc<Mutex<String>>,
}

impl Default for Map2MdlSync {
    fn default() -> Self {
        Self {
            stdout: Arc::new(Mutex::new("Idle".to_string())),
        }
    }
}

impl Map2MdlSync {
    pub fn stdout(&self) -> &Arc<Mutex<String>> {
        &self.stdout
    }
}

#[derive(Default, Debug)]
pub struct Map2Mdl {
    options: Map2MdlOptions,
    /// Converts a .map file
    ///
    /// Can be used with marked_entity option to convert specifically [`GCHIMP_MAP2MDL_ENTITY_NAME`]
    map: Option<PathBuf>,
    /// Converts a provided entity text
    ///
    /// Entity should be a worldbrush, meaning it is part of entity 0
    entity: Option<String>,
    wads: Vec<PathBuf>,
    sync: Option<Map2MdlSync>,
}

impl Map2Mdl {
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

    pub fn ignore_nodraw(&mut self, v: bool) -> &mut Self {
        self.options.ignore_nodraw = v;
        self
    }

    pub fn add_wad(&mut self, v: &Path) -> &mut Self {
        self.wads.push(v.to_path_buf());
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

    /// Converts a .map file
    ///
    /// Can be used with marked_entity option to convert specifically [`GCHIMP_MAP2MDL_ENTITY_NAME`]
    pub fn map(&mut self, v: &str) -> &mut Self {
        self.map = PathBuf::from(v).into();
        self
    }

    /// Converts a provided entity text
    ///
    /// Entity should be a worldbrush, meaning it is part of entity 0
    pub fn entity(&mut self, v: &str) -> &mut Self {
        self.entity = v.to_owned().into();
        self
    }

    pub fn marked_entity(&mut self, v: bool) -> &mut Self {
        self.options.marked_entity = v;
        self
    }

    pub fn sync(&mut self, v: Map2MdlSync) -> &mut Self {
        self.sync = v.into();
        self
    }

    fn log(&self, what: &str) {
        println!("{}", what);

        if let Some(sync) = &self.sync {
            let mut lock = sync.stdout.lock().unwrap();
            *lock += what;
            *lock += "\n";
        }
    }

    fn convert_from_triangles(
        &self,
        smd_triangles: &[Triangle],
        textures_used: &HashSet<String>,
        // output path would be where the model ends up with
        // output path should be the .mdl file
        output_path: &Path,
        // resource path is where qc smd and textures file are stored
        // usually it should be the .map file
        resource_path: &Path,
        move_to_origin: bool,
        export_resource: bool,
    ) -> eyre::Result<usize> {
        // before splitting smd, we need to check if we want to split model
        let model_count = textures_used.len() / MAX_GOLDSRC_MODEL_TEXTURE_COUNT + 1;
        let textures_used_vec = textures_used.iter().collect::<Vec<&String>>();

        // if we dont create any new resource, this is enough
        if !export_resource {
            self.log("Skipped creating qc, smd, and model files");

            return Ok(model_count);
        }

        let mut main_smd = Smd::new_basic();

        // if no ORIGIN brush given, then the centroid will be the centroid of the brush
        let origin_brush_triangles = smd_triangles
            .iter()
            .filter(|tri| tri.material == ORIGIN_TEXTURE)
            .cloned()
            .collect::<Vec<Triangle>>();

        smd_triangles
            .iter()
            .filter(|tri| {
                if self.options.ignore_nodraw {
                    !NO_RENDER_TEXTURE.contains(&tri.material.as_str())
                } else {
                    true
                }
            })
            .for_each(|tri| {
                main_smd.add_triangle(tri.clone());
            });

        let brush_centroid = if origin_brush_triangles.is_empty() {
            find_centroid(&main_smd).unwrap()
        } else {
            find_centroid_from_triangles(&origin_brush_triangles).unwrap()
        };

        if move_to_origin {
            move_by(&mut main_smd, -brush_centroid);
        }

        // DO NOT ADD EXTENSION HERE, YET
        // it should be the last step
        // because we are still processing over some data
        // add_bitmap_extension_to_texture(&mut main_smd);

        // the format of the file will follow
        // smd: <output><model index>_<smd_index>
        // mdl/qc: <output><model index>
        // even if there is 1 modela nd 1 smd, too bad

        // idle smd
        // every model uses the same idle smd so that's ok
        Smd::new_basic().write(resource_path.with_file_name("idle.smd"))?;

        let smd_and_qc_res = (0..model_count)
            .map(|model_index| {
                // "0" suffix is only added when there are more than 1 model count
                let model_name = if model_count == 1 {
                    output_path
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                } else {
                    format!(
                        "{}{}",
                        output_path.file_stem().unwrap().to_str().unwrap(),
                        model_index,
                    )
                };

                let current_model_textures = textures_used_vec
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

                        smd.write(resource_path.with_file_name(smd_name))?;

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
                new_qc.add_cd(resource_path.parent().unwrap().to_str().unwrap());
                new_qc.add_cd_texture(resource_path.parent().unwrap().to_str().unwrap());

                current_model_textures.iter().for_each(|texture| {
                    // for the best results, TexTile does convert to compliant transparent texture
                    if texture.starts_with("{") {
                        new_qc.add_texrendermode(
                            // ".bmp" is required
                            format!("{}.bmp", texture).as_str(),
                            qc::RenderMode::Masked,
                        );
                    }
                });

                for smd_index in 0..smd_count {
                    new_qc.add_body(
                        format!("studio{}", smd_index).as_str(),
                        format!("{}_{}", model_name, smd_index).as_str(),
                        false,
                        None,
                    );
                }

                new_qc.add_sequence("idle", "idle", vec![]);

                let qc_out_path = resource_path.with_file_name(format!("{}.qc", model_name));

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

        Ok(model_count)
    }

    fn maybe_export_texture(
        &self,
        textures_used: &HashSet<String>,
        wads: &[&Wad],
        simple_wads: &SimpleWad,
    ) -> eyre::Result<()> {
        // if all good, export texture if needed
        if self.options.export_texture {
            self.log(format!("Exporting {} texture(s)", textures_used.len()).as_str());

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
                    // textures will be exported inside studiomdl folder if convert entity
                    let out_path_file = if let Some(map) = &self.map {
                        map
                    } else if let Some(studiomdl) = &self.options.studiomdl {
                        studiomdl
                    } else {
                        unreachable!()
                    };

                    export_texture(
                        wads[simple_wads.get(tex).unwrap().wad_file_index()],
                        tex,
                        out_path_file.with_file_name(tex),
                    )
                })
                .find_any(|res| res.is_err())
            {
                return err;
            }
        }

        Ok(())
    }

    pub fn work(&mut self) -> eyre::Result<()> {
        self.log("Starting Map2Mdl");
        self.log("Checking settings");

        if self.map.is_none() && self.entity.is_none() {
            return err!("No input provided.");
        }

        if self.options.studiomdl.is_none() {
            return err!("No studiomdl.exe supplied.");
        }

        #[cfg(target_os = "linux")]
        if self.options.wineprefix.is_none() {
            return err!("No WINEPREFIX supplied.");
        }

        // very convoluted error propagating
        let map_file = self.map.as_ref().map(Map::from_file);

        if let Some(Err(err)) = &map_file {
            return err!("Cannot parse map file: {}", err);
        }

        let mut map_file = if let Some(map_file) = map_file {
            self.log("Converting map");
            map_file.ok()
        } else {
            None
        };

        // repeating the convoluted error propagating
        let entity_entity = self
            .entity
            .as_ref()
            .map(|entity| Map::from_text(entity).map(|res| res.entities[0].clone()));

        if let Some(Err(err)) = &entity_entity {
            return err!("Cannot parse entity: {}", err);
        }

        let entity_entity = if let Some(entity_entity) = entity_entity {
            self.log("Converting entity");
            entity_entity.ok()
        } else {
            None
        };

        // more checking even though this is very redundant
        if map_file.is_none() && entity_entity.is_none() {
            if self.map.is_some() {
                return err!("Cannot parse map file.");
            }

            if self.entity.is_some() {
                return err!("Cannot parse entity text.");
            }
        }

        if let Some(entity) = &entity_entity {
            if !entity.attributes.contains_key("wad") {
                return err!("Provided entity does not contain \"wad\" key. Make sure entity is a worldbrush.");
            }
        }

        // now we talking about something different
        let valid_autopickup_wad_for_map = map_file.is_some()
            && map_file.as_ref().unwrap().entities[0] // always entity 0
                .attributes
                .get("wad")
                .is_some_and(|paths| !paths.is_empty());

        let valid_autopickup_wad_for_entity = entity_entity.is_some()
            && entity_entity
                .as_ref()
                .unwrap()
                .attributes
                .get("wad") // worldbrush only becuase it is entity 0
                .is_some_and(|paths| !paths.is_empty());

        // now we are collecting wad files
        let valid_autopickup_wad = self.options.auto_pickup_wad
            && (valid_autopickup_wad_for_map || valid_autopickup_wad_for_entity);

        if self.wads.is_empty() && (!valid_autopickup_wad) {
            return err!("Cannot pick up any WAD files.");
        }

        let wads_res = if !self.wads.is_empty() {
            self.wads
                .iter()
                .map(Wad::from_file)
                .collect::<Vec<eyre::Result<Wad>>>()
        } else if valid_autopickup_wad {
            let hashset = if let Some(entity_entity) = &entity_entity {
                &entity_entity.attributes
            } else if let Some(map_file) = &map_file {
                &map_file.entities[0].attributes
            } else {
                unreachable!()
            };

            let wad = hashset.get("wad").unwrap();

            self.log(format!("Auto pickup WAD found: {}", wad).as_str());

            wad.split_terminator(";")
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

        // now we create simple wad presentation because finding data is more annoying than making new data
        let wads = wads_res
            .iter()
            .filter_map(|res| res.as_ref().ok())
            .collect::<Vec<&Wad>>();

        let simple_wads: SimpleWad = wads.as_slice().into();

        // check for missing textures
        let textures_used = if let Some(map) = &map_file {
            if self.options.marked_entity {
                map.entities
                    .iter()
                    .filter(|entity| {
                        entity
                            .attributes
                            .get("classname")
                            .is_some_and(|classname| classname == GCHIMP_MAP2MDL_ENTITY_NAME)
                    })
                    .map(textures_used_in_entity)
                    .fold(HashSet::<String>::new(), |mut acc, e| {
                        acc.extend(e);
                        acc
                    })
            } else {
                textures_used_in_map(map)
            }
        } else if let Some(entity) = &entity_entity {
            textures_used_in_entity(entity)
        } else {
            unreachable!()
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

        // this is the main part
        // if we have a map file, we either convert the whole map or just selected entitities
        // if we don't have a map, we might have an entity pasted in the GUI part
        if let Some(map) = &mut map_file {
            if self.options.marked_entity {
                self.log(format!("Converting {} only", GCHIMP_MAP2MDL_ENTITY_NAME).as_str());

                // check if the the info entity is there
                let gchimp_info_entity = &map.entities[check_gchimp_info_entity(map)?];

                if gchimp_info_entity
                    .attributes
                    .get("options")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
                    & 1
                    == 0
                {
                    println!(
                        "map2mdl is not enabled as specified in {}",
                        GCHIMP_INFO_ENTITY
                    );
                    return Ok(());
                }

                let map2mdl_export_resource = gchimp_info_entity
                    .attributes
                    .get("options")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
                    & 2
                    != 0;

                if !map2mdl_export_resource {
                    println!(
                        "\
map2mdl model export is not enabled as specified in {}. \
This means gchimp will not export textures and convert entities into models. \
However, it will still turn {} into model displaying entities such as cycler_sprite.",
                        GCHIMP_INFO_ENTITY, GCHIMP_MAP2MDL_ENTITY_NAME
                    );

                    println!("Skipped creating textures")
                } else {
                    self.maybe_export_texture(&textures_used, &wads, &simple_wads)?;
                }

                let output_base_path =
                    PathBuf::from(gchimp_info_entity.attributes.get("hl_path").unwrap())
                        .join(gchimp_info_entity.attributes.get("gamedir").unwrap());

                let mut marked_entities = map
                    .entities
                    .par_iter_mut()
                    .enumerate()
                    .filter(|(_, entity)| {
                        entity
                            .attributes
                            .get("classname")
                            .is_some_and(|classname| classname == GCHIMP_MAP2MDL_ENTITY_NAME)
                    })
                    .collect::<Vec<(usize, &mut Entity)>>();

                // check if all entities have "output" key
                let missing_output_name = marked_entities
                    .iter()
                    .filter(|(_, entity)| !entity.attributes.contains_key("output"))
                    .map(|(index, _)| index)
                    .collect::<Vec<&usize>>();

                if !missing_output_name.is_empty() {
                    return err!(
                        "Missing output name for some entities: {:?}",
                        missing_output_name
                    );
                }

                // check if the output path exists
                let nonexistent_output = marked_entities
                    .iter()
                    .filter_map(|(_, entity)| entity.attributes.get("output"))
                    .filter_map(|output| PathBuf::from(output).parent().map(|what| what.to_owned()))
                    .map(|output| output_base_path.join(output))
                    .filter(|output| !output.exists())
                    .collect::<Vec<PathBuf>>();

                if !nonexistent_output.is_empty() {
                    self.log(format!("Some paths to output model do not exist: {:?}\nThis means the directory is not created. Attempting to create directory.", nonexistent_output).as_str());

                    let create_dir_err = nonexistent_output
                        .iter()
                        .filter_map(|path| fs::create_dir_all(path).err())
                        .collect::<Vec<_>>();

                    if !create_dir_err.is_empty() {
                        return err!(
                            "Fail to create directories for output models: {}",
                            create_dir_err
                                .into_iter()
                                .fold(String::new(), |acc, e| acc + e.to_string().as_str())
                        );
                    }
                }

                // TOOD: this might be redundant if we realy do a brush entity, phase out bitch
                // check if entity brush really has brush
                let has_no_brushes = marked_entities
                    .iter()
                    .filter(|(_, entity)| entity.brushes.is_none())
                    .map(|(index, _)| *index)
                    .collect::<Vec<usize>>();

                if !has_no_brushes.is_empty() {
                    return err!("Some entities don't have brushes: {:?}", has_no_brushes);
                }

                // triangulate
                self.log(
                    format!(
                        "Running convex hull clipping algorithm over {} entities",
                        marked_entities.len()
                    )
                    .as_str(),
                );
                let (ok, err): (Vec<Vec<Triangle>>, Vec<eyre::Report>) =
                    marked_entities.par_iter().partition_map(|(_, entity)| {
                        let res = entity_to_triangulated_smd(entity, &simple_wads, false);

                        if let Ok(ok) = res {
                            Either::Left(ok)
                        } else if let Err(err) = res {
                            Either::Right(err)
                        } else {
                            unreachable!()
                        }
                    });

                self.log(
                    format!(
                        "Created {} triangles over {} entities",
                        ok.iter().fold(0, |acc, e| acc + e.len()),
                        marked_entities.len()
                    )
                    .as_str(),
                );

                if !err.is_empty() {
                    return err!(
                        "Cannot triangulate all marked entities: {}",
                        err.into_iter()
                            .fold(String::new(), |acc, e| acc + e.to_string().as_str())
                    );
                }

                let model_entity_default = "cycler_sprite".to_string();

                // create the models
                // due to some rust stuff, this cannot be done in parallel (first)
                self.log(format!("Creating {} models", marked_entities.len()).as_str());

                let (map2mdl_ok, map2mdl_err): (Vec<eyre::Result<usize>>, _) = marked_entities
                    .iter()
                    .zip(ok.iter()) // safe to assume this is all in order?
                    .map(|((_, entity), smd_triangles)| {
                        // this output path will contain the .mdl extension
                        let output_path =
                            output_base_path.join(entity.attributes.get("output").unwrap());
                        let resource_path = self.map.as_ref().unwrap();

                        self.convert_from_triangles(
                            smd_triangles,
                            &textures_used,
                            output_path.as_path(),
                            resource_path,
                            // always move to origin
                            // this makes the centroid more consistent when we move it back with entity
                            true,
                            // if no export then the function returns right away
                            map2mdl_export_resource,
                        )
                    })
                    .partition(|res| res.is_ok());

                if !map2mdl_err.is_empty() {
                    return err!(
                        "Cannot create model: {}",
                        map2mdl_err.into_iter().fold(String::new(), |acc, e| acc
                            + e.unwrap_err().to_string().as_str())
                    );
                }

                let map2mdl_ok = map2mdl_ok
                    .into_iter()
                    .map(|what| what.unwrap())
                    .collect::<Vec<usize>>();

                // change entity and maybe create clip brush
                // TODO verify TB's layer stuffs
                self.log(format!("Modifying {}", self.map.as_ref().unwrap().display()).as_str());

                let to_insert = marked_entities
                    .iter_mut()
                    .zip(ok.iter()) // safe to assume this is all in order?
                    .zip(map2mdl_ok)
                    .filter_map(|(((entity_index, entity), smd_triangles), model_count)| {
                        // two cases for to change
                        // if there is clip brush, then the original brush will be chagned into func_detail and clip texture
                        // then entity is inserted
                        // if not clip brush, will delete the brush of the entity and replace the entity in place
                        // doing that won't change the map too much ,especially tb layer
                        // the result of this iterator will be the model entity to be inserted in case we have clip option chosen

                        // 0: noclip
                        // 1: precise
                        // 2: box
                        let clip_type = entity
                            .attributes
                            .get("cliptype")
                            .map(|s| s.parse::<usize>().unwrap_or(0))
                            .unwrap_or(0)
                            .clamp(0, 2);

                        // cycler_sprite
                        // env_sprite
                        // cycler
                        let model_classname = entity
                            .attributes
                            .get("model_entity")
                            .unwrap_or(&model_entity_default)
                            .to_owned();
                        // some more info
                        let model_origin = find_centroid_from_triangles(smd_triangles).unwrap();
                        let model_origin =
                            format!("{} {} {}", model_origin.x, model_origin.y, model_origin.z);

                        // "0" suffix is only added when there are more than 1 model count
                        let model_modelname0 = if model_count == 1 {
                            entity.attributes.get("output").unwrap().to_owned()
                        } else {
                            entity
                                .attributes
                                .get("output")
                                .unwrap()
                                .replace(".mdl", "0.mdl")
                        };
                        let model_angles = "0 0 0".to_string();

                        let mut entities_to_insert: Vec<Entity> = vec![];

                        // "0" suffix is only added when there are more than 1 model count
                        (1..(model_count)).for_each(|model_index| {
                            let curr_model_name = model_modelname0
                                .replace("0.mdl", format!("{}.mdl", model_index).as_str());

                            let new_entity = Entity {
                                attributes: Attributes::from([
                                    ("classname".to_string(), model_classname.to_owned()),
                                    ("origin".to_owned(), model_origin.to_owned()),
                                    ("angles".to_owned(), model_angles.to_owned()),
                                    ("model".to_owned(), curr_model_name),
                                ]),
                                brushes: None,
                            };

                            entities_to_insert.push(new_entity);
                        });

                        if clip_type == 1 {
                            let mut clip_brush_entity = entity.clone();

                            if let Some(brushes) = &mut clip_brush_entity.brushes {
                                brushes.iter_mut().for_each(|brush| {
                                    brush.planes.iter_mut().for_each(|plane| {
                                        plane.texture_name = CLIP_TEXTURE.to_string();
                                    })
                                })
                            }

                            clip_brush_entity.attributes.clear();

                            clip_brush_entity
                                .attributes
                                .insert("classname".to_string(), "func_detail".to_string());

                            // remove origin because brush entity
                            // otherwise the editor would confuse
                            // maybe need to remove more in the future if there's problems
                            clip_brush_entity.attributes.remove("origin");

                            entities_to_insert.push(clip_brush_entity);
                        }

                        // for all cliptype, the original brush would turn into the model entity
                        // doing this will make the model entity inherit original passed in values
                        entity.brushes = None;
                        entity
                            .attributes
                            .insert("classname".to_owned(), model_classname);
                        entity.attributes.insert("origin".to_owned(), model_origin);
                        entity.attributes.insert("angles".to_owned(), model_angles);
                        entity
                            .attributes
                            .insert("model".to_owned(), model_modelname0);

                        // now specific to clip_type = 2
                        // we need to insert a brush later
                        if clip_type == 2 {
                            let [mins, maxs] = find_mins_maxs(smd_triangles);
                            let new_brush = brush_from_mins_maxs(&mins, &maxs, "CLIP");
                            let new_brush_entity = Entity {
                                attributes: Attributes::from([(
                                    "classname".to_string(),
                                    "func_detail".to_owned(),
                                )]),
                                brushes: vec![new_brush].into(),
                            };

                            entities_to_insert.push(new_brush_entity);
                        }

                        if entities_to_insert.is_empty() {
                            None
                        } else {
                            Some((*entity_index, entities_to_insert))
                        }
                    })
                    .collect::<Vec<(usize, Vec<Entity>)>>();

                // lastly, insert entities
                to_insert
                    .into_iter()
                    .rev()
                    .for_each(|(entity_index, entities)| {
                        // insert + 1 because we insert right after the entity
                        entities.into_iter().for_each(|entity| {
                            map.entities.insert(entity_index + 1, entity);
                        })
                    });

                // lastly^2 write the map file
                self.log(format!("Writing new {}", self.map.as_ref().unwrap().display()).as_str());
                map.write(self.map.as_ref().unwrap())?;
            } else {
                self.log("Converting whole map file");

                self.maybe_export_texture(&textures_used, &wads, &simple_wads)?;

                // just convert the whole map, very simple
                self.log("Running convex hull clipping algorithm");
                let smd_triangles = map_to_triangulated_smd(map, &simple_wads, false)?;
                self.log(format!("Created {} triangles", smd_triangles.len()).as_str());

                let output_path = self.map.as_ref().unwrap();

                self.convert_from_triangles(
                    &smd_triangles,
                    &textures_used,
                    output_path,
                    output_path,
                    self.options.move_to_origin,
                    true,
                )?;
            }
        } else if let Some(entity) = &entity_entity {
            self.log("Converting entity");

            self.maybe_export_texture(&textures_used, &wads, &simple_wads)?;

            self.log("Running convex hull clipping algorithm");
            let smd_triangles = entity_to_triangulated_smd(entity, &simple_wads, false)?;
            self.log(format!("Created {} triangles", smd_triangles.len()).as_str());

            let output_path = self
                .options
                .studiomdl
                .as_ref()
                .unwrap()
                .with_file_name("map2mdl.mdl");

            self.log("Creating model");
            self.convert_from_triangles(
                &smd_triangles,
                &textures_used,
                output_path.as_path(),
                output_path.as_path(),
                self.options.move_to_origin,
                true,
            )?;
        } else {
            unreachable!()
        };

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map("/home/khang/gchimp/examples/map2prop/map.map")
            .work()
            .unwrap();
    }

    #[test]
    fn run2() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map("/home/khang/gchimp/examples/map2prop/map2.map")
            .work()
            .unwrap();
    }

    #[test]
    fn arte_twist() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map("/home/khang/gchimp/examples/map2prop/arte_spin/arte_spin.map")
            .work()
            .unwrap();
    }

    #[test]
    fn sphere() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map("/home/khang/gchimp/examples/map2prop/sphere.map")
            .work()
            .unwrap();
    }

    #[test]
    fn sphere2() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map("/home/khang/gchimp/examples/map2prop/sphere2.map")
            .work()
            .unwrap();
    }

    #[test]
    fn marked_1() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .map("/home/khang/gchimp/examples/map2prop/marked/marked.map")
            .marked_entity(true)
            .work()
            .unwrap();
    }

    #[test]
    fn entity() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .entity("\
// entity 0
{
\"mapversion\" \"220\"
\"wad\" \"/home/khang/map_compiler/sdhlt.wad;/home/khang/map_compiler/devtextures.wad\"
\"classname\" \"worldspawn\"
\"_tb_mod\" \"cstrike;cstrike_downloads\"
\"_tb_def\" \"external:/home/khang/map_compiler/combined.fgd\"
// brush 0
{
( -64 0 80 ) ( -64 -64 128 ) ( -64 -64 64 ) devcrate64 [ 0 -1 0 0 ] [ 0 0 -1 16 ] 0 1 1
( -64 -64 128 ) ( 64 -64 128 ) ( 64 -64 64 ) devcrate64 [ 1 0 0 0 ] [ 0 0 -1 16 ] 0 1 1
( 64 -64 64 ) ( 64 0 64 ) ( -64 0 64 ) devcrate64 [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -64 0 80 ) ( 64 0 80 ) ( 64 -64 128 ) devcrate64 [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 0 64 ) ( 64 0 80 ) ( -64 0 80 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 16 ] 0 1 1
( 64 -64 128 ) ( 64 0 80 ) ( 64 0 64 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 16 ] 0 1 1
}
// brush 1
{
( -64 64 128 ) ( -64 0 80 ) ( -64 0 64 ) devcrate64 [ 0 -1 0 0 ] [ 0 0 -1 16 ] 0 1 1
( -64 0 80 ) ( 64 0 80 ) ( 64 0 64 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 16 ] 0 1 1
( -64 64 128 ) ( 64 64 128 ) ( 64 0 80 ) devcrate64 [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 0 64 ) ( 64 64 64 ) ( -64 64 64 ) devcrate64 [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 64 ) ( 64 64 128 ) ( -64 64 128 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 16 ] 0 1 1
( 64 0 80 ) ( 64 64 128 ) ( 64 64 64 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 16 ] 0 1 1
}
// brush 2
{
( -89.3725830020305 0 60.117749006091444 ) ( -89.3725830020305 64 60.117749006091444 ) ( -179.88225099390857 64 150.62741699796953 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 -41.705627 ] [ 0 -1 0 0 ] 0 1 1
( -134.62741699796953 64 195.88225099390857 ) ( -168.5685424949238 0 161.9411254969543 ) ( -179.88225099390857 0 150.62741699796953 ) devcrate64 [ 0 -1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686288 ] 0 1 1
( -168.5685424949238 0 161.9411254969543 ) ( -78.05887450304573 0 71.4314575050762 ) ( -89.3725830020305 0 60.117749006091444 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 -41.705627 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686288 ] 45 1 1
( -89.3725830020305 64 60.117749006091444 ) ( -44.11774900609145 64 105.37258300203048 ) ( -134.62741699796953 64 195.88225099390857 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 -41.705627 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686289 ] 315 1 1
( -134.62741699796953 64 195.88225099390857 ) ( -44.11774900609145 64 105.37258300203048 ) ( -78.05887450304573 0 71.4314575050762 ) devcrate64 [ 0.7071067811865475 0 -0.7071067811865477 41.705627 ] [ 0 -1 0 0 ] 27.91369 1 1
( -78.05887450304573 0 71.4314575050762 ) ( -44.11774900609145 64 105.37258300203048 ) ( -89.3725830020305 64 60.117749006091444 ) devcrate64 [ 0 1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686288 ] 0 1 1
}
// brush 3
{
( -89.3725830020305 -64 60.117749006091444 ) ( -89.3725830020305 0 60.117749006091444 ) ( -179.88225099390857 0 150.62741699796953 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 -41.705627 ] [ 0 -1 0 0 ] 0 1 1
( -168.5685424949238 0 161.9411254969543 ) ( -134.62741699796953 -64 195.88225099390857 ) ( -179.88225099390857 -64 150.62741699796953 ) devcrate64 [ 0 -1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686288 ] 0 1 1
( -134.62741699796953 -64 195.88225099390857 ) ( -44.11774900609145 -64 105.37258300203048 ) ( -89.3725830020305 -64 60.117749006091444 ) devcrate64 [ 0.7071067811865475 0 -0.7071067811865477 41.705627 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686289 ] 45 1 1
( -89.3725830020305 0 60.117749006091444 ) ( -78.05887450304573 0 71.4314575050762 ) ( -168.5685424949238 0 161.9411254969543 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 -41.705627 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686288 ] 315 1 1
( -168.5685424949238 0 161.9411254969543 ) ( -78.05887450304573 0 71.4314575050762 ) ( -44.11774900609145 -64 105.37258300203048 ) devcrate64 [ 0.7071067811865475 0 -0.7071067811865477 41.705627 ] [ 0 -1 0 0 ] 332.0863 1 1
( -44.11774900609145 -64 105.37258300203048 ) ( -78.05887450304573 0 71.4314575050762 ) ( -89.3725830020305 0 60.117749006091444 ) devcrate64 [ 0 1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 -4.686288 ] 0 1 1
}
}
")
            .work()
            .unwrap();
    }

    #[test]
    fn edge_case_cut_cube_diagonally_first() {
        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(false)
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .studiomdl(PathBuf::from("/home/khang/gchimp/dist/studiomdl.exe").as_path())
            .entity("\
// Game: Half-Life
// Format: Valve
// entity 0
{
\"mapversion\" \"220\"
\"wad\" \"/home/khang/map_compiler/sdhlt.wad;/home/khang/map_compiler/surf_ben10.wad\"
\"classname\" \"worldspawn\"
\"_tb_mod\" \"cstrike;cstrike_downloads\"
\"_tb_def\" \"external:/home/khang/map_compiler/combined.fgd\"
// brush 0
{
( -32 32 -32 ) ( -32 -32 32 ) ( 96 -32 32 ) benowykingkurti [ -1 0 0 0 ] [ 0 -0.7071067811865476 0.7071067811865476 0 ] 0 1 1
( -32 -32 16 ) ( -32 -31 16 ) ( -32 -32 17 ) benowykingkurti [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -16 32 32 ) ( -16 33 32 ) ( -15 32 32 ) benowykingkurti [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -16 32 32 ) ( -15 32 32 ) ( -16 32 33 ) benowykingkurti [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 32 32 32 ) ( 32 32 33 ) ( 32 33 32 ) benowykingkurti [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}

")
            .work()
            .unwrap();
    }
}
