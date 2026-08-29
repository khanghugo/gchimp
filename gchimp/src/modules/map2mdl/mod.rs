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
use wad::{error::WadError, types::Wad};

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
            entity_to_triangulated_smd,
        },
        misc::{f64_3_to_u8_3, parse_triplet},
        smd_stuffs::{find_aabb_center_from_triangles, find_mins_maxs, maybe_split_triangles},
        wad_stuffs::SimpleWad,
    },
};

/// Converts entire .map to model
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

/// Converts a TrenchBroom-copied entity brush to a model
///
/// The map is saved at <entity_option.output>/models.mdl
///
/// It is recommended to use gchimp binary path for <entity_option.output>
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
    if map.entities.is_empty() {
        return Err(Map2MdlError::EmptyMap);
    }

    let (simple_wad, wads) = generate_wad_info(map)?;

    // there is map_to_triangulated_smd
    // but it should not be used because that will pass the entire map geometry to the special texture processing function
    // if CONTENTWATER is used, the entire map is duplicated
    let (triangles, extra_textures) = map
        .entities
        .iter()
        .filter(|x| x.brushes.is_some()) // only work on entities with brush
        .map(|entity| {
            convert_entity_to_triangles(
                || {
                    entity_to_triangulated_smd(entity, &simple_wad, false).map_err(|x| {
                        Map2MdlError::FailToConvertBrushToTriangles {
                            reason: x.to_string(),
                        }
                    })
                },
                &map.entities[0],
                entity_option,
            )
        })
        .try_fold((Vec::new(), Vec::new()), |(mut tris, mut texs), res| {
            let (t, x) = res?;
            tris.extend(t);
            texs.extend(x);
            Ok::<_, Map2MdlError>((tris, texs))
        })?;

    let triangle_chunks = partition_mesh2(triangles);
    let mdls = generate_models(
        entity_option,
        &simple_wad,
        &wads,
        triangle_chunks,
        extra_textures,
    );

    mdls.into_par_iter().for_each(|(file_name, mdl)| {
        // there is no need for gchimp_info in this case
        // usually, the caller should supply the output path in that option as welel
        let output_path = entity_option.output.with_file_name(file_name);
        mdl.write_to_file(output_path)
            .expect("cannot write model file"); // TODO: too fatigued to handle error here
    });

    Ok(())
}

// model count is independent from entity count
// it is possible that entity count exceeds model count but not the other way around
// if there are 2 models, then there should be at least 2 entities
// if there are 2 models, it is possible to have 3 entites where the third entity is the brush entity
type Map2MdlConvertEntityResult = (Vec<(String, mdl::Mdl)>, Vec<map::Entity>, Map2MdlOption);

/// Converts all gchimp_map2mdl entities to individual .mdl(s)
///
/// This is the way to use Map2Mdl with map editor as it also does some IO stuffs
// maybe in the future, should break this function down so others can call the common part
// namingly, getting .mdl(s) and final .map file
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

    println!("Found {} {} entities", entities.len(), MAP2MDL_ENTITY_NAME);

    // generate wad info
    let (simple_wad, wads) = generate_wad_info(&map)?;

    // convert all gchimp_map2mdl
    let convert_results = entities
        .par_iter()
        .map(|entity| convert_map2mdl_entity(&map, entity, &simple_wad, &wads))
        .collect::<Result<Vec<_>, Map2MdlError>>()?;

    // convert all used textures to upper case
    let total_model_count = convert_results.iter().fold(0, |acc, e| e.0.len() + acc);
    let total_entity_count = convert_results.iter().fold(0, |acc, e| e.1.len() + acc);

    println!(
        "Writing ({}) models and ({}) entities",
        total_model_count, total_entity_count
    );

    // flatten the result so it is easier to process
    let mut map2mdl_results = Vec::with_capacity(total_model_count);
    let mut entities_to_insert = Vec::with_capacity(total_entity_count);

    for (models, entities, option) in convert_results {
        map2mdl_results.push((models, option));
        entities_to_insert.push(entities);
    }

    // insert new entities
    // have to stucture entity as Vec<Vec<Entity>>
    // because it preserves the entity order
    {
        // it is surely sorted, the code is here just in case
        assert_eq!(entities_indices, {
            let mut clone = entities_indices.clone();
            clone.sort();
            clone
        });
    }

    entities_to_insert
        .into_iter()
        .zip(entities_indices)
        .rev() // insert reverse
        .for_each(|(entities, insert_index)| {
            // no need for the entity at insert_index, just remove it
            // it is the original gchimp_map2mdl entity
            // make sure to delete it only once
            map.entities.remove(insert_index);

            entities.into_iter().for_each(|entity| {
                map.entities.insert(insert_index, entity);
            });
        });

    // write models
    let output_base_path = PathBuf::from(gchimp_info.hl_path()).join(gchimp_info.gamedir());

    let error_reports: Vec<_> = map2mdl_results
        .into_par_iter()
        .flat_map(|(mdls, option)| {
            let base_output_path = output_base_path.join(option.output);

            mdls.par_iter()
                .flat_map(|(file_name, mdl)| {
                    let output_path = base_output_path.with_file_name(file_name);

                    // try to create the folder to store model
                    let Some(model_parent) = output_path.parent() else {
                        let output_path_display = output_path.to_string_lossy().to_string();

                        return Some((
                            output_path,
                            Map2MdlError::GenericError {
                                value: format!(
                                    "Cannot create folder for model {}",
                                    output_path_display
                                ),
                            },
                        ));
                    };

                    if let Err(e) = std::fs::create_dir_all(model_parent) {
                        let model_parent = model_parent.to_path_buf();

                        return Some((
                            output_path,
                            Map2MdlError::GenericError {
                                value: format!(
                                    "Cannot create folder {} ({})",
                                    model_parent.display(),
                                    e
                                ),
                            },
                        ));
                    }

                    match mdl.write_to_file(&output_path) {
                        Ok(_) => {
                            println!("Writing {}", output_path.display());
                            None
                        }
                        Err(e) => Some((
                            output_path,
                            Map2MdlError::GenericError {
                                value: e.to_string(),
                            },
                        )),
                    }
                })
                .collect::<Vec<(PathBuf, Map2MdlError)>>()
        })
        .collect();

    if !error_reports.is_empty() {
        return Err(Map2MdlError::GenericError {
            value: format!("Failed to write models: {:?}", error_reports),
        });
    }

    map.write(map_path.as_ref())
        .map_err(|x| Map2MdlError::GenericError {
            value: x.to_string(),
        })?;

    Ok(())
}

fn generate_wad_info(map: &map::Map) -> Result<(SimpleWad, Vec<Wad>), Map2MdlError> {
    let entity0 = map.entities.first().ok_or(Map2MdlError::EmptyMap)?;
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

    let wads = wads_paths
        .into_iter()
        .map(Wad::from_file)
        .collect::<Result<Vec<_>, WadError>>()
        .map_err(|x| Map2MdlError::GenericError {
            value: x.to_string(),
        })?;

    let simple_wad: SimpleWad = wads.as_slice().into();
    let simple_wad = simple_wad.uppercase(); // must use uppercase for compatibility

    Ok((simple_wad, wads))
}

type CustomTexture = (String, GoldSrcBmp);

fn convert_entity_to_triangles(
    make_triangles: impl Fn() -> Result<Vec<Triangle>, Map2MdlError>,
    entity: &Entity,
    option: &Map2MdlOption,
) -> Result<(Vec<Triangle>, Vec<CustomTexture>), Map2MdlError> {
    let mut triangles = make_triangles()?;

    // as celshade takes precedence
    // does celshade stuffs
    let mut celshade_custom_texture = vec![];

    if option
        .spawnflags
        // use intersects here because AND
        .intersects(Map2MdlEntitySpawnflag::AsCelShade | Map2MdlEntitySpawnflag::WithCelShade)
    {
        // SAFETY: the previous entity_to_triangulated_smd should throw error if there are not brushes in this entity
        let mut celshade_res =
            generate_celshade(entity.brushes.as_ref().unwrap(), &option.celshade_options)?;

        if option
            .spawnflags
            .contains(Map2MdlEntitySpawnflag::AsCelShade)
        {
            // as celshade will remove original brush
            // as celshade takes precedence
            triangles = celshade_res.mesh;
        } else if option
            .spawnflags
            .contains(Map2MdlEntitySpawnflag::WithCelShade)
        {
            triangles.append(&mut celshade_res.mesh);
        }

        celshade_custom_texture.push((celshade_res.texture_name, celshade_res.image));
    }

    let triangles = process_special_textures(option, triangles)?;

    Ok((triangles, celshade_custom_texture))
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
    wads: &[Wad],
) -> Result<Map2MdlConvertEntityResult, Map2MdlError> {
    let entity_option = verify_and_get_entity_options(entity)?;
    let (triangles, celshade_texture) = convert_entity_to_triangles(
        || {
            entity_to_triangulated_smd(entity, simple_wad, false).map_err(|x| {
                Map2MdlError::FailToConvertBrushToTriangles {
                    reason: x.to_string(),
                }
            })
        },
        entity,
        &entity_option,
    )?;

    // move the mesh
    let (model_world_origin, triangles) = modify_mesh_origin(map, &entity_option, triangles)?;

    let exts = find_mins_maxs(&triangles);

    // parition into multiple group of smd because of max model texture count
    let triangle_chunks = partition_mesh2(triangles);

    // now write chunk to actual model
    let mdls = generate_models(
        &entity_option,
        simple_wad,
        wads,
        triangle_chunks,
        celshade_texture,
    );

    // generate model entities to insert to the map
    let model_names: Vec<&str> = mdls.iter().map(|x| x.0.as_str()).collect();
    let entities = touch_entities(
        &entity_option,
        entity,
        model_world_origin,
        &model_names,
        exts,
    );

    Ok((mdls, entities, entity_option))
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
    let maybe_target_origin = if let Some(target_origin) = &option.target_origin {
        // exclusive keyword "origin" to make it 0 0 0 without adding an extra entity
        if target_origin == "origin" {
            Some(DVec3::ZERO)
        } else {
            if let Some(entity_attributes) = map.entities.iter().find(|&entity| {
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
            }
        }
    } else {
        None
    };

    // at the moment, the model is offset from origin (0, 0), need to move it back to center
    let brush_world_centroid = if !origin_brush_triangles.is_empty() {
        find_aabb_center_from_triangles(&origin_brush_triangles).unwrap()
    } else if let Some(target_origin) = maybe_target_origin {
        target_origin
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

// This function should be called for EACH ENTITY
// There is CONTENTWATER processing and that duplicates geometry
// TODO: this should be better isolated to PER BRUSH, but for now it is OK
fn process_special_textures(
    option: &Map2MdlOption,
    mut triangles: Vec<smd::Triangle>,
) -> Result<Vec<smd::Triangle>, Map2MdlError> {
    let is_content_water = triangles
        .iter()
        .any(|triangle| triangle.material == CONTENTWATER_TEXTURE);

    let mut extra_triangles = vec![];

    // first remove all no render textures
    triangles.retain(|tri| !NoRenderTexture.contains(tri.material.as_str()));

    // then process special textures like CONTENTWATER
    triangles.iter_mut().for_each(|tri| {
        if option
            .spawnflags
            .contains(Map2MdlEntitySpawnflag::ReverseNormals)
        {
            tri.vertices.iter_mut().for_each(|vertex| {
                vertex.norm *= -1.;
            });
        }

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
    wads: &[Wad],
    triangle_chunks: Vec<(Vec<String>, Vec<Triangle>)>,
    custom_texture: Vec<CustomTexture>,
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
                let (image, palette, dimensions) = {
                    if let Some(lookup) = simple_wad.get(texture_name) {
                        let (wad_index, file_index) = lookup.index();

                        let texture = &wads[wad_index].entries[file_index];
                        let wad::types::FileEntry::MipTex(texture) = &texture.file_entry else {
                            unreachable!("Texture `{}` is not a miptex", texture_name);
                        };

                        (
                            texture.mip_images[0].get_bytes().to_owned(),
                            texture.palette.get_bytes().to_owned(),
                            (texture.width, texture.height),
                        )
                    } else if let Some((_, custom_texture)) = custom_texture
                        .iter()
                        .find(|(custom_texture_name, _)| custom_texture_name == texture_name)
                    {
                        // if cannot find from look up, look at custom texture instead
                        // TODO: custom_texture should be consumed here, no debate, but this is fine too
                        (
                            custom_texture.image.clone(),
                            custom_texture.palette.clone(),
                            custom_texture.dimensions,
                        )
                    } else {
                        unreachable!("Should have textures. Are you using textures that are not inside WAD files?")
                    }
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
                        image,
                        palette,
                        dimensions,
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

// Process entity.
// Should own the entity for a more correct memory model
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
        if let Some(x) = base_entity.classname_mut() {
            *x = option.model_entity.clone()
        }
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
        let mut model_displaying_entity = base_entity.clone();

        let path = option.output.with_file_name(model_name);

        model_displaying_entity
            .attributes
            .insert("model".into(), path.display().to_string());

        // also no brush
        model_displaying_entity.brushes = None;

        entities_to_insert.push(model_displaying_entity);
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

fn generate_celshade(
    brushes: &[Brush],
    options: &Map2MdlEntityCelShadeOption,
) -> Result<CelShadeResult, Map2MdlError> {
    let celshade_texture_name = format!(
        "{:03}{:03}{:03}",
        options.color[0], options.color[1], options.color[2]
    );

    let mut simple_wad = SimpleWad::new();
    simple_wad.insert(
        &celshade_texture_name,
        (0, 0),
        (CELSHADE_TEXTURE_DIMENSION, CELSHADE_TEXTURE_DIMENSION),
    );

    let triangulated_smds = brushes
        .iter()
        .cloned()
        // expand the original brush
        .map(|mut x| {
            x.expand_mut(options.distance as f64);
            x
        })
        // triangulate the brush
        .map(|brush| {
            brush_to_triangulated_smd(
                &brush,
                &simple_wad,
                false,
                true, // important, ignore WADs
            )
            .map_err(|x| Map2MdlError::FailToConvertBrushToTriangles {
                reason: x.to_string(),
            })
        })
        .collect::<Result<Vec<Vec<smd::Triangle>>, Map2MdlError>>()?
        .into_iter()
        .flatten()
        // remove no render textures
        .filter(|triangle| !NoRenderTexture.contains(&triangle.material))
        // some processing
        .map(|mut triangle| {
            // flip winding order so it is inside out
            triangle.flip_winding_order_mut();
            // change texture name to celshade color
            triangle.material = celshade_texture_name.clone();

            triangle
        })
        .collect();

    const CELSHADE_TEXTURE_DIMENSION: u32 = 16;

    let wad_image = GoldSrcBmp {
        image: vec![0; (CELSHADE_TEXTURE_DIMENSION * CELSHADE_TEXTURE_DIMENSION) as usize],
        palette: vec![options.color; 8],
        dimensions: (CELSHADE_TEXTURE_DIMENSION, CELSHADE_TEXTURE_DIMENSION),
    };

    Ok(CelShadeResult {
        mesh: triangulated_smds,
        texture_name: celshade_texture_name,
        image: wad_image,
    })
}
