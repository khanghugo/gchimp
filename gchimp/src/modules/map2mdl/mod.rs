use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use common::{
    constants::{
        CLIP_TEXTURE, CONTENTWATER_TEXTURE, MAX_GOLDSRC_MODEL_TEXTURE_COUNT, NoRenderTexture,
        ORIGIN_TEXTURE, RenderMode,
    },
    img_stuffs::GoldSrcBmp,
};
use glam::DVec3;
use map::{Brush, Entity};
use smd::Triangle;
use studiomdl::StudioMdl;
use wad::types::Wad;

use rayon::prelude::*;

pub mod entity;
pub mod types;

use crate::{
    gchimp_info::GchimpInfo,
    modules::map2mdl::{
        entity::{
            MAP2MDL_ATTR_CELSHADE_COLOR, MAP2MDL_ATTR_CELSHADE_DISTANCE, MAP2MDL_ATTR_CLIPTYPE,
            MAP2MDL_ATTR_MODEL_ENTITY, MAP2MDL_ATTR_OUTPUT, MAP2MDL_ATTR_TARGET_ORIGIN,
            MAP2MDL_ENTITY_NAME,
        },
        types::{
            Map2MdlEntityCelShadeOption, Map2MdlEntityCliptype, Map2MdlEntitySpawnflag,
            Map2MdlError, Map2MdlOption,
        },
    },
    utils::{
        map_stuffs::{
            brush_from_mins_maxs, brush_to_triangulated_smd, convert_used_texture_to_uppercase,
            entity_to_triangulated_smd, map_to_triangulated_smd,
        },
        misc::{f64_3_to_u8_3, parse_triplet},
        smd_stuffs::{find_aabb_center_from_triangles, find_mins_maxs, maybe_split_triangles},
        wad_stuffs::SimpleWad,
    },
};

pub fn convert_entire_map(
    map_path: impl Into<PathBuf> + AsRef<Path>,
    entity_option: &Map2MdlOption,
) -> Result<(), Map2MdlError> {
    // basic validation to make sure that there is a path to write
    // usually, the path in this case points to gchimp binary
    // and then replaced with the model file name
    if !entity_option.output.is_file() || !entity_option.output.exists() {
        return Err(Map2MdlError::GenericError {
            value: "Output path does not exist or is not a file".into(),
        });
    }

    let map = map::Map::from_file(map_path).map_err(|op| Map2MdlError::GenericError {
        value: op.to_string(),
    })?;

    convert_map(&map, entity_option)
}

pub fn convert_world_brush_entity(
    entity_text: &str, // entity text copied from trenchbroom is actually the map file but with only one entity
    entity_option: &Map2MdlOption,
) -> Result<(), Map2MdlError> {
    let map = map::Map::from_text(entity_text).map_err(|_| Map2MdlError::GenericError {
        value: "Cannot parse entity text".into(),
    })?;

    convert_map(&map, entity_option)
}

// DRY for convert world brush entity and entire map
fn convert_map(map: &map::Map, entity_option: &Map2MdlOption) -> Result<(), Map2MdlError> {
    let (simple_wad, wads) = generate_wad_info(map)?;

    let triangles = map_to_triangulated_smd(map, &simple_wad, false).map_err(|x| {
        Map2MdlError::GenericError {
            value: x.to_string(),
        }
    })?;

    let triangles = process_special_textures(entity_option, &simple_wad, triangles)?;
    let triangle_chunks = partition_mesh2(triangles);
    let mdls = generate_models(entity_option, &simple_wad, &wads, triangle_chunks);

    mdls.into_par_iter().for_each(|(file_name, mdl)| {
        // there is no need for gchimp_info in this case
        // usually, the caller should supply the output path in that option as welel
        let output_path = entity_option.output.with_file_name(file_name);
        mdl.write_to_file(output_path)
            .expect("cannot write model file"); // TODO: too fatigued to handle error here
    });
    Ok(())
}

type Map2MdlConvertEntityResult = (Vec<(String, mdl::Mdl, map::Entity)>, Map2MdlOption);

pub fn convert_all_map2mdl_entities(
    map_path: impl Into<PathBuf> + AsRef<Path>,
) -> Result<(), Map2MdlError> {
    let mut map =
        map::Map::from_file(map_path.as_ref()).map_err(|op| Map2MdlError::GenericError {
            value: op.to_string(),
        })?;

    // convert all map used textures to uppercase for best compatibility
    convert_used_texture_to_uppercase(&mut map);

    // find gchimp_info
    let gchimp_info = GchimpInfo::from_map(&map).map_err(|x| Map2MdlError::GenericError {
        value: x.to_string(),
    })?;

    // find map2mdl entities
    let entities_indices = map.get_entities_by_classname_all(MAP2MDL_ENTITY_NAME);
    let entities: Vec<&Entity> = entities_indices.iter().map(|x| &map.entities[*x]).collect();

    // generate wad info
    let (simple_wad, wads) = generate_wad_info(&map)?;

    // convert all gchimp_map2mdl
    let convert_results: Vec<_> = entities
        .par_iter()
        .map(|entity| convert_map2mdl_entity(&map, entity, &simple_wad, &wads))
        .collect();

    // clean up result
    let convert_results: Vec<(Vec<(String, mdl::Mdl, Entity)>, Map2MdlOption)> = {
        let mut res = vec![];

        for i in convert_results {
            let what = i?;
            res.push(what);
        }

        res
    };

    // delete older map2mdl entities
    let entities_indices = entities_indices;

    // no need to sort here because it should be sorted
    assert_eq!(entities_indices, {
        let mut clone = entities_indices.clone();

        clone.sort();

        clone
    });

    for index in entities_indices.iter().rev() {
        map.entities.remove(*index);
    }

    // convert all used textures to upper case

    // insert new entities and write models
    // separate result entities from mdl
    let mut map2mdl_results = Vec::with_capacity(convert_results.len());
    let mut entities_to_insert = Vec::with_capacity(convert_results.len());

    for (entity_result, option) in convert_results {
        let mut new_inner = Vec::with_capacity(entity_result.len());
        let mut new_inner_entities = Vec::with_capacity(entity_result.len());

        for (name, mdl, entity) in entity_result {
            new_inner.push((name, mdl));
            new_inner_entities.push(entity);
        }

        map2mdl_results.push((new_inner, option));
        entities_to_insert.push(new_inner_entities);
    }

    // insert new entities
    // have to stucture entity as Vec<Vec<Entity>>
    // because it preserves the entity order
    entities_to_insert
        .into_iter()
        .zip(entities_indices)
        .rev()
        .for_each(|(entities, insert_index)| {
            entities.into_iter().for_each(|entity| {
                map.entities.insert(insert_index, entity);
            });
        });

    // write models
    let output_base_path = PathBuf::from(gchimp_info.hl_path()).join(gchimp_info.gamedir());

    let error_paths: Vec<PathBuf> = map2mdl_results
        .into_par_iter()
        .flat_map(|(mdls, option)| {
            let base_output_path = output_base_path.join(option.output);

            mdls.par_iter()
                .flat_map(|(file_name, mdl)| {
                    let output_path = base_output_path.with_file_name(file_name);

                    match mdl.write_to_file(&output_path) {
                        Ok(_) => None,
                        Err(_) => Some(output_path),
                    }
                })
                .collect::<Vec<PathBuf>>()
        })
        .collect();

    if !error_paths.is_empty() {
        return Err(Map2MdlError::GenericError {
            value: format!("Failed to write models: {:?}", error_paths),
        });
    }

    map.write(map_path.as_ref())
        .map_err(|x| Map2MdlError::GenericError {
            value: x.to_string(),
        })?;

    Ok(())
}

fn generate_wad_info(map: &map::Map) -> Result<(SimpleWad, Vec<Wad>), Map2MdlError> {
    let entity0 = map.entities.get(0).ok_or(Map2MdlError::EmptyMap)?;
    let wad_value = entity0
        .attributes
        .get("wad")
        .ok_or(Map2MdlError::NoWadKey)?;
    let wads_paths = wad_value
        .split_terminator(";")
        .map(|path_as_str| {
            #[allow(unused_mut)]
            let mut path_as_string = path_as_str.to_owned();

            let path = Path::new(path_as_str);

            if !path.exists() {
                #[cfg(target_os = "windows")]
                {
                    (b'A'..b'Z').for_each(|l: u8| {
                        let chr = l as char;

                        let new_path_string = format!("{chr}:{}", path_as_str);
                        let new_path = Path::new(&new_path_string);

                        if new_path.exists() {
                            path_as_string = new_path_string;
                        }
                    });
                }
            }

            path_as_string.clone()
        })
        .collect::<Vec<String>>();

    let wads_results: Vec<_> = wads_paths
        .into_iter()
        .map(|path| Wad::from_file(path))
        .collect();

    let wads = {
        let mut result = vec![];

        for i in wads_results {
            result.push(i.map_err(|x| Map2MdlError::GenericError {
                value: x.to_string(),
            })?);
        }

        result
    };

    let simple_wad: SimpleWad = wads.as_slice().into();
    let simple_wad = simple_wad.uppercase(); // must use uppercase for compatibility

    Ok((simple_wad, wads))
}

/// This function does not mutate map file
///
/// It is up to the caller to clean up old gchimp_map2mdl entities
///
/// Returns: Vec<(model name, mdl, map entity)>
fn convert_map2mdl_entity(
    map: &map::Map,
    entity: &Entity,
    simple_wad: &SimpleWad,
    wads: &Vec<Wad>,
) -> Result<Map2MdlConvertEntityResult, Map2MdlError> {
    let entity_option = verify_and_get_entity_options(entity)?;
    let triangles = entity_to_triangulated_smd(entity, simple_wad, false).map_err(|x| {
        Map2MdlError::GenericError {
            value: x.to_string(),
        }
    })?;

    // move the mesh
    let (model_world_origin, triangles) = modify_mesh_origin(map, &entity_option, triangles)?;

    // process special texture
    // either remove NULL and alike or make it CONTENTWATER correct
    let triangles = process_special_textures(&entity_option, simple_wad, triangles)?;

    let exts = find_mins_maxs(&triangles);

    // parition into multiple group of smd because of max model texture count
    let triangle_chunks = partition_mesh2(triangles);

    // now write chunk to actual model
    let mdls = generate_models(&entity_option, simple_wad, wads, triangle_chunks);

    // generate model entities to insert to the map
    let model_names: Vec<&str> = mdls.iter().map(|x| x.0.as_str()).collect();
    let entities = touch_entities(
        &entity_option,
        entity,
        model_world_origin,
        &model_names,
        exts,
    );

    Ok((
        mdls.into_iter()
            .zip(entities)
            .map(|((name, mdl), entity)| (name, mdl, entity))
            .collect(),
        entity_option,
    ))
}

fn modify_mesh_origin(
    map: &map::Map,
    option: &Map2MdlOption,
    mut triangles: Vec<smd::Triangle>,
) -> Result<(DVec3, Vec<smd::Triangle>), Map2MdlError> {
    let origin_brush_triangles = triangles
        .iter()
        .filter(|tri| tri.material == ORIGIN_TEXTURE)
        .cloned()
        .collect::<Vec<smd::Triangle>>();
    let maybe_target_origin = if let Some(target_origin) = &option.target_origin
        && let Some(entity_attributes) = map.entities.iter().find(|&entity| {
            entity
                .targetname()
                .is_some_and(|targetname_curr| targetname_curr == target_origin)
        }) {
        let res = entity_attributes
            .origin()
            .ok_or(Map2MdlError::GenericError {
                value: "Target entity does not have origin".into(),
            })?;

        Some(res)
    } else {
        None
    };

    // at the moment, the model is offset from origin (0, 0), need to move it back to center
    let brush_world_centroid = if !origin_brush_triangles.is_empty() {
        find_aabb_center_from_triangles(&origin_brush_triangles).unwrap()
    } else if let Some(target_origin) = maybe_target_origin {
        target_origin.into()
    } else {
        find_aabb_center_from_triangles(&triangles).unwrap()
    };

    // do the actual moving
    triangles.iter_mut().for_each(|triangle| {
        triangle.vertices.iter_mut().for_each(|vertex| {
            vertex.pos -= brush_world_centroid;
        })
    });

    Ok((brush_world_centroid, triangles))
}

// TODO: this should be done per brush basis, not per entire entity
// the expectation is that for any entities that use special textures like CONTENTWATER,
// faces shall be duplicated pertaining to that brush only
// currently the code treats everything as one single brush
// this means the output of conversion code should be Vec<Vec<Triangle>>, not Vec<Triangle>
// then I must have to think how long do I keep that structure
fn process_special_textures(
    option: &Map2MdlOption,
    simple_wad: &SimpleWad,
    mut triangles: Vec<smd::Triangle>,
) -> Result<Vec<smd::Triangle>, Map2MdlError> {
    // if an entity has CONTENTWATER texture, then the entire entity
    let is_content_water = simple_wad.0.keys().any(|x| x == CONTENTWATER_TEXTURE);

    let mut extra_triangles = vec![];

    triangles
        .iter()
        .filter(|tri| !NoRenderTexture.contains(tri.material.as_str()))
        .for_each(|tri| {
            let mut new_tri = tri.clone();

            if option
                .spawnflags
                .contains(Map2MdlEntitySpawnflag::ReverseNormals)
            {
                new_tri.vertices.iter_mut().for_each(|vertex| {
                    vertex.norm *= -1.;
                });
            }

            extra_triangles.push(new_tri);

            // no way to skip special textures
            if is_content_water {
                let mut new_tri = tri.clone();

                new_tri.vertices.iter_mut().for_each(|vertex| {
                    // reverse normal just to be safe
                    vertex.norm *= -1.;
                });

                new_tri.vertices.swap(0, 1);
                extra_triangles.push(new_tri);
            }
        });

    triangles.append(&mut extra_triangles);

    Ok(triangles)
}

fn partition_mesh2(triangles: Vec<smd::Triangle>) -> Vec<(Vec<String>, Vec<smd::Triangle>)> {
    let mut by_material: HashMap<String, Vec<smd::Triangle>> = HashMap::new();
    for tri in triangles {
        by_material
            .entry(tri.material.clone())
            .or_default()
            .push(tri);
    }

    let mut by_material: Vec<_> = by_material.into_iter().collect();

    by_material
        .chunks_mut(MAX_GOLDSRC_MODEL_TEXTURE_COUNT)
        .map(|chunk| {
            let mut chunk_textures = Vec::with_capacity(chunk.len());
            let mut chunk_triangles = Vec::new();

            for (mat, tris) in chunk {
                chunk_textures.push(std::mem::take(mat));
                chunk_triangles.append(tris);
            }

            (chunk_textures, chunk_triangles)
        })
        .collect()
}

// input
// Vec<([textures used in this chunk], [triangle])>
// output
// Vec<(model name, mdl)>
fn generate_models(
    option: &Map2MdlOption,
    simple_wad: &SimpleWad,
    wads: &Vec<Wad>,
    triangle_chunks: Vec<(Vec<String>, Vec<Triangle>)>,
) -> Vec<(String, mdl::Mdl)> {
    let model_basename = option
        .output
        .file_stem()
        .expect("no file stem??")
        .to_string_lossy();

    triangle_chunks
        .into_iter()
        .enumerate()
        .map(|(model_index, (texture_names, triangles))| {
            let mut studiomdl = StudioMdl::new();
            let split_meshes = maybe_split_triangles(triangles);

            // need to split the mesh to comply with max vertices count
            split_meshes
                .into_iter()
                .enumerate()
                .for_each(|(mesh_idx, mesh)| {
                    studiomdl.add_bodypart((format!("gchimp{}", mesh_idx), mesh));
                });

            // add texture
            texture_names.iter().for_each(|texture_name| {
                let lookup = simple_wad
                    .get(texture_name)
                    .expect("used texture does not match simple wad");
                let (wad_index, file_index) = lookup.index();

                let texture = &wads[wad_index].entries[file_index];
                let wad::types::FileEntry::MipTex(texture) = &texture.file_entry else {
                    unreachable!("Texture `{}` is not a miptex", texture_name);
                };

                let mut texture_flag = mdl::TextureFlag::NOMIPS;

                texture_flag.set(
                    mdl::TextureFlag::FLATSHADE,
                    option
                        .spawnflags
                        .contains(Map2MdlEntitySpawnflag::FlatShade),
                );
                texture_flag.set(
                    mdl::TextureFlag::MASKED,
                    option.rendermode == RenderMode::Solid,
                );
                texture_flag.set(
                    mdl::TextureFlag::ADDITIVE,
                    option.rendermode == RenderMode::Additive,
                );

                studiomdl.add_texture((
                    texture_name.to_owned(),
                    GoldSrcBmp {
                        image: texture.mip_images[0].get_bytes().to_owned(),
                        palette: texture.palette.get_bytes().to_owned(),
                        dimensions: (texture.width, texture.height),
                    },
                    texture_flag,
                ));
            });

            let model_name = format!("{}{:02}.mdl", model_basename, model_index);

            studiomdl.set_model_name(&model_name);

            (
                model_name,
                studiomdl.compile().expect("failed to build model"),
            )
        })
        .collect()
}

fn touch_entities(
    option: &Map2MdlOption,
    entity: &Entity,
    entity_new_origin: DVec3,
    model_names: &[&str],
    (min, max): (DVec3, DVec3),
) -> Vec<Entity> {
    let mut entities_to_insert = vec![];

    // generate entities
    // result entity will inherit everything from base entity
    // but some entries are modified or even deleted so the final entity
    // can be pure
    let base_entity = {
        let mut base_entity = entity.clone();

        // set basic stuffs
        base_entity
            .classname_mut()
            .map(|x| *x = option.model_entity.clone());
        base_entity
            .attributes
            .insert("angles".into(), "0 0 0".into());
        base_entity.attributes.insert(
            "origin".into(),
            format!(
                "{} {} {}",
                entity_new_origin.x, entity_new_origin.y, entity_new_origin.z,
            ),
        );

        // clear spawnflags
        // TODO: maybe rollback again and don't use spawnflags for gchimp_map2mdl
        // because it conflicts with result entity spawnflags
        // it should be better that we can set spawnflags but maybe not
        // unsure if this should even be a TODO
        base_entity.attributes.remove("spawnflags");

        base_entity.attributes.remove(MAP2MDL_ATTR_OUTPUT);
        base_entity.attributes.remove(MAP2MDL_ATTR_CLIPTYPE);
        base_entity.attributes.remove(MAP2MDL_ATTR_MODEL_ENTITY);
        base_entity.attributes.remove(MAP2MDL_ATTR_TARGET_ORIGIN);
        base_entity
            .attributes
            .remove(MAP2MDL_ATTR_CELSHADE_DISTANCE);
        base_entity.attributes.remove(MAP2MDL_ATTR_CELSHADE_COLOR);

        base_entity
    };

    // first, generate clip entity if there is any
    // doing this first because we won't mess up with entity brush data
    match option.cliptype {
        Map2MdlEntityCliptype::NoClip => {}
        Map2MdlEntityCliptype::SameAsBrush => {
            let mut clip_brush_entity = base_entity.clone();

            clip_brush_entity.to_func_detail();

            if let Some(brushes) = &mut clip_brush_entity.brushes {
                brushes.iter_mut().for_each(|brush| {
                    brush.planes.iter_mut().for_each(|plane| {
                        plane.texture_name = map::TextureName::new(CLIP_TEXTURE.to_owned());
                    });
                });
            }

            entities_to_insert.push(clip_brush_entity);
        }
        Map2MdlEntityCliptype::BiggestBox => {
            let mut clip_brush_entity = base_entity.clone();

            clip_brush_entity.to_func_detail();

            let new_brush = brush_from_mins_maxs(min, max, "CLIP");

            clip_brush_entity.brushes = Some(vec![new_brush]);
            entities_to_insert.push(clip_brush_entity);
        }
    }

    // now generate all the model displayinng entities
    // the only difference is the "model" key
    model_names.iter().for_each(|model_name| {
        let mut to_insert = base_entity.clone();

        let path = option.output.with_file_name(model_name);

        to_insert
            .attributes
            .insert("model".into(), path.display().to_string());

        entities_to_insert.push(to_insert);
    });

    entities_to_insert
}

fn verify_and_get_entity_options(entity: &Entity) -> Result<Map2MdlOption, Map2MdlError> {
    let output = entity
        .attributes
        .get(MAP2MDL_ATTR_OUTPUT)
        .ok_or(Map2MdlError::NoOutput)?;

    if !output.ends_with(".mdl") {
        return Err(Map2MdlError::OutputNotMdl);
    }

    let model_entity = entity
        .attributes
        .get(MAP2MDL_ATTR_MODEL_ENTITY)
        .ok_or(Map2MdlError::NoModelEntity)?;

    let cliptype = entity
        .attributes
        .get(MAP2MDL_ATTR_CLIPTYPE)
        .ok_or(Map2MdlError::NoCliptype)?;

    let cliptype = Map2MdlEntityCliptype::try_from(cliptype.as_str())?;

    let target_origin = entity.attributes.get(MAP2MDL_ATTR_TARGET_ORIGIN);

    let spawnflags: Map2MdlEntitySpawnflag = entity.spawnflags().unwrap_or(0).into();

    let celshade_color = entity
        .attributes
        .get(MAP2MDL_ATTR_CELSHADE_COLOR)
        .and_then(|v| parse_triplet(v).ok())
        .map(f64_3_to_u8_3)
        .unwrap_or([0, 0, 0]);

    let celshade_distance = entity
        .attributes
        .get(MAP2MDL_ATTR_CELSHADE_DISTANCE)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(4.);

    let rendermode = entity
        .rendermode()
        .and_then(|x| x.parse::<u32>().ok())
        .and_then(|x| RenderMode::try_from(x).ok())
        .unwrap_or_default();

    Ok(Map2MdlOption {
        output: output.into(),
        model_entity: model_entity.into(),
        cliptype,
        target_origin: target_origin.cloned(),
        spawnflags,
        celshade_options: Map2MdlEntityCelShadeOption {
            color: celshade_color,
            distance: celshade_distance,
        },
        rendermode,
    })
}

struct CelShadeResult {
    mesh: Vec<smd::Triangle>,
    texture_name: String,
    image: GoldSrcBmp,
}

fn generate_celshade(brushes: &Vec<Brush>, options: Map2MdlEntityCelShadeOption) -> CelShadeResult {
    let texture_name = format!(
        "{:03}{:03}{:03}",
        options.color[0], options.color[1], options.color[2]
    );

    let mut simple_wad = SimpleWad::new();
    simple_wad.insert(
        &texture_name,
        (0, 0),
        (CELSHADE_TEXTURE_DIMENSION, CELSHADE_TEXTURE_DIMENSION),
    );

    let triangulated_smds: Vec<smd::Triangle> = brushes
        .iter()
        .cloned()
        .map(|mut x| {
            x.planes
                .iter_mut()
                .for_each(|x| x.texture_name = map::TextureName::new(texture_name.clone()));

            x.expand(options.distance as f64);

            x
        })
        .map(|brush| {
            brush_to_triangulated_smd(&brush, &simple_wad, false)
                .expect("cannot convert celshade brush to triangulated smd")
        })
        .flatten()
        .collect();

    const CELSHADE_TEXTURE_DIMENSION: u32 = 16;

    let wad_image = GoldSrcBmp {
        image: vec![0; (CELSHADE_TEXTURE_DIMENSION * CELSHADE_TEXTURE_DIMENSION) as usize],
        palette: vec![options.color; 8],
        dimensions: (CELSHADE_TEXTURE_DIMENSION, CELSHADE_TEXTURE_DIMENSION),
    };

    CelShadeResult {
        mesh: triangulated_smds,
        texture_name,
        image: wad_image,
    }
}
