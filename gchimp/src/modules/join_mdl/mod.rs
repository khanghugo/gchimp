use std::path::{Path, PathBuf};

use glam::DVec3;
use map::Map;
use mdl::Mdl;

use crate::{
    gchimp_info::{GchimpInfo, GchimpInfoOption},
    modules::join_mdl::{
        entity::{
            JMDL_ATTR_MODEL_ENTITY, JMDL_ATTR_OUTPUT, JMDL_BRUSH_ENTITY_NAME, JMDL_ENTITY_NAME,
        },
        error::JMdlError,
    },
    utils::{
        map_stuffs::{brush_to_solid3d, solid_3d_to_convex_hull},
        mdl_stuffs::{JoinMdlsParameters, join_mdls_with_affine_transformation},
        simple_calculs::{Point3D, Solid3D},
    },
};

mod entity;
mod error;

struct JMdlWorkOrder {
    jmdl_entity_idx: usize,
    model_entities_indices: Vec<usize>,
    brush_solid: Option<Vec<Solid3D>>,
}

pub fn join_model(map: &mut Map) -> Result<usize, JMdlError> {
    // verifies that there is gchimp_info
    // jmdl uses gchimp_info to find where the model is
    // will only search inside `basegame` and `basegame_downloads`
    let gchimp_info = GchimpInfo::from_map(map)?;

    // is enabled beause i want this to be standard
    if !gchimp_info.spawnflags().contains(GchimpInfoOption::JoinMDL) {
        println!("JoinMDL is not enabled.");
        return Ok(0);
    }

    let game_dirs = if let Some(stripped) = gchimp_info.gamedir().strip_suffix("_downloads") {
        vec![
            "valve".into(),
            stripped.to_owned(),
            format!("{}_downloads", stripped),
        ]
    } else {
        let gamedir = gchimp_info.gamedir();

        vec![
            "valve".into(),
            gamedir.to_owned(),
            format!("{}_downloads", gamedir),
        ]
    };

    // verify that the gamedir exists oor just not
    let model_prefixes = game_dirs
        .iter()
        .map(|x| Path::new(gchimp_info.hl_path()).join(x))
        .filter(|x| x.exists())
        .collect::<Vec<PathBuf>>();
    let output_prefix = Path::new(gchimp_info.hl_path()).join(gchimp_info.gamedir());

    let mut work_orders: Vec<JMdlWorkOrder> = vec![];

    // go over the map first to find "gchimp_mdl" and "trigger_gchimp_mdl"
    // gchimp_mdl first
    {
        for jmdl_entity_idx in map.get_entities_by_classname_all(JMDL_ENTITY_NAME) {
            // list of model entities that point to our gchimp_jmdl
            // this just has better ergonomics
            // if gchimp_jmdl contains the model targetname instead, you have to
            // name the model and then insert it to the gchimp_mdl
            let jmdl_targetname = map.entities[jmdl_entity_idx]
                .targetname()
                .cloned()
                .ok_or(JMdlError::NoTargetName)?;

            let model_entities_indices = map
                .entities
                .iter()
                .enumerate()
                .filter(|(_, ent)| ent.target() == Some(&jmdl_targetname))
                .map(|(idx, _)| idx)
                .collect::<Vec<usize>>();

            if model_entities_indices.is_empty() {
                continue;
            }

            work_orders.push(JMdlWorkOrder {
                jmdl_entity_idx,
                model_entities_indices,
                brush_solid: None,
            });
        }
    }

    // trigger_gchimp_mdl
    {
        const MODEL_ENTITIES_TO_CHECK: &[&str] = &["cycler_sprite", "env_sprite"];

        // now, sadly, iterate over all model entities
        let all_model_displaying_entities: Vec<usize> = MODEL_ENTITIES_TO_CHECK
            .iter()
            .flat_map(|x| map.get_entities_by_classname_all(x))
            .collect();

        let all_model_displaying_entities_origin: Vec<(usize, DVec3)> =
            all_model_displaying_entities
                .into_iter()
                .filter_map(|x| map.entities[x].origin().map(|origin| (x, origin)))
                .collect();

        for jmdl_brush_entity_idx in map.get_entities_by_classname_all(JMDL_BRUSH_ENTITY_NAME) {
            let Some(brushes) = &map.entities[jmdl_brush_entity_idx].brushes else {
                continue;
            };

            let mut model_entity_list = vec![];
            // for each brush, check if any of the models are inside the brush
            let solids: Vec<_> = brushes.iter().map(brush_to_solid3d).collect();

            for (model_entity_idx, origin) in &all_model_displaying_entities_origin {
                if solids.iter().any(|x| x.contains_point((*origin).into())) {
                    model_entity_list.push(*model_entity_idx);
                }
            }

            if model_entity_list.is_empty() {
                continue;
            }

            work_orders.push(JMdlWorkOrder {
                jmdl_entity_idx: jmdl_brush_entity_idx,
                model_entities_indices: model_entity_list,
                brush_solid: Some(solids),
            });
        }
    }

    // now work
    // the work count here is just for the output to have something interesting to say
    let mut work_count = 0;

    // delete entities later when all the entities are processed
    // otherwise, the entity index no longer works
    let mut entities_to_delete = vec![];

    for work_order in work_orders {
        let JMdlWorkOrder {
            jmdl_entity_idx,
            mut model_entities_indices,
            brush_solid,
        } = work_order;

        // verify output valu8e
        let output_relative_path = map.entities[jmdl_entity_idx]
            .attributes
            .get(JMDL_ATTR_OUTPUT)
            .cloned()
            .ok_or(JMdlError::NoOutput)?;
        let output_absolute_path = output_prefix.join(&output_relative_path);

        {
            if output_absolute_path.extension().is_some_and(|x| x != "mdl") {
                return Err(JMdlError::OutputNotMdl {
                    name: output_relative_path,
                });
            }

            // create directory to be sure
            if let Some(path) = output_absolute_path.parent() {
                std::fs::create_dir_all(path).map_err(|e| JMdlError::IOError { source: e })?;
            }
        }

        // model paths
        let model_paths = model_entities_indices
            .iter()
            .filter_map(|&idx| map.entities[idx].model())
            .collect::<Vec<&String>>();

        // verify that all models exists
        // model_full_paths.len() is not necessarily the same as model_paths.len()
        let model_full_paths = model_paths
            .iter()
            .filter_map(|path| {
                model_prefixes
                    .iter()
                    .map(|prefix| prefix.join(path))
                    .find(|full_path| full_path.exists())
                    .or_else(|| {
                        println!("Cannot find any model for `{}`", path);
                        None
                    })
            })
            .collect::<Vec<PathBuf>>();

        // open all models
        let mdls = model_full_paths
            .iter()
            .filter_map(|path| match Mdl::open_from_file(path) {
                Ok(x) => Some(x),
                Err(x) => {
                    println!("Failed to open {:?} {}", path, x);
                    None
                }
            })
            .collect::<Vec<Mdl>>();

        // gather affine transformations

        // find where the jmdl entity origin is. Can be very complicated if needed
        let jmdl_entity_origin = jmdl_entity_origin(map, jmdl_entity_idx, &brush_solid)?;

        let translations = model_entities_indices
            .iter()
            .map(|index| map.entities[*index].origin().unwrap_or(DVec3::ZERO))
            .map(|origin| origin - jmdl_entity_origin)
            .collect::<Vec<DVec3>>();
        let rotations = model_entities_indices
            .iter()
            // PITCH YAW ROLL // -Y Z X
            .map(|&index| map.entities[index].angles().unwrap_or(DVec3::ZERO))
            .map(|rotation| DVec3 {
                x: rotation.z.to_radians(),
                y: -rotation.x.to_radians(),
                z: rotation.y.to_radians(),
            })
            .collect::<Vec<DVec3>>();
        let scales = model_entities_indices
            .iter()
            // PITCH YAW ROLL // -Y Z X
            .map(|&index| map.entities[index].scale().unwrap_or(1.))
            .collect::<Vec<f64>>();
        let sequences = model_entities_indices
            .iter()
            .map(|&index| map.entities[index].sequence().unwrap_or(0))
            .collect::<Vec<u32>>();

        // the model
        let mut combined_model = join_mdls_with_affine_transformation(
            &mdls,
            JoinMdlsParameters {
                translations,
                rotations,
                scales,
                sequences,
            },
        )?;

        // now, replace gchimp_jmdl with model entity
        let model_entity_name = map.entities[jmdl_entity_idx]
            .attributes
            .get(JMDL_ATTR_MODEL_ENTITY)
            .cloned()
            .unwrap_or("cycler_sprite".into());

        map.entities[jmdl_entity_idx]
            .attributes
            .entry("classname".into())
            .and_modify(|x| *x = model_entity_name);

        map.entities[jmdl_entity_idx]
            .attributes
            .insert("model".into(), output_relative_path);

        map.entities[jmdl_entity_idx].brushes = None; // guarantee no brushes

        // affirm origin, in case of brush, it wont have origin unless this
        map.entities[jmdl_entity_idx].attributes.insert(
            "origin".into(),
            format!(
                "{:.4} {:.4} {:.4}",
                jmdl_entity_origin.x, jmdl_entity_origin.y, jmdl_entity_origin.z
            ),
        );

        // at this point, new model entity should inherit all key and values

        // removes gchimp_jmdl owned key and values
        map.entities[jmdl_entity_idx]
            .attributes
            .remove(JMDL_ATTR_MODEL_ENTITY);
        map.entities[jmdl_entity_idx]
            .attributes
            .remove(JMDL_ATTR_OUTPUT);

        // delete all other models
        entities_to_delete.append(&mut model_entities_indices);

        // write the model
        // must have model name so it can be precached
        combined_model.set_name(
            &output_absolute_path
                .file_name()
                .unwrap()
                .display()
                .to_string(),
        );
        combined_model.rebuild_data_for_export();

        combined_model.write_to_file(output_absolute_path)?;

        work_count += 1;
    }

    // no, actually delete at the end
    // otherwise, the entity index is shuffled.
    entities_to_delete.sort();
    entities_to_delete.iter().rev().for_each(|idx| {
        map.entities.remove(*idx);
    });

    Ok(work_count)
}

fn jmdl_entity_origin(
    map: &mut Map,
    jmdl_entity_idx: usize,
    brush_solid: &Option<Vec<Solid3D>>,
) -> Result<DVec3, JMdlError> {
    if let Some(solids) = brush_solid {
        // hard path, we have a brush that covers entities

        // first, check if there is "target" key. That can be our origin.
        if let Some(target_origin_name) = map.entities[jmdl_entity_idx].target() {
            // easy path, yet another entity origin
            if let Some(point_entity) = map.get_entity_by_targetname(target_origin_name) {
                if let Some(origin) = map.entities[point_entity].origin() {
                    Ok(origin)
                } else {
                    Err(JMdlError::BrushTargetNotPointEntity {
                        name: target_origin_name.to_owned(),
                    })
                }
            } else {
                Err(JMdlError::BrushNoTarget {
                    name: target_origin_name.to_owned(),
                })
            }
        } else {
            // hard path, find the extents and use the middle as origin
            let convex_hulls: Vec<_> = solids
                .iter()
                .map(|x| solid_3d_to_convex_hull(x, false))
                .collect();

            if let Some(x) = convex_hulls
                .iter()
                .filter_map(|hull| hull.get_bounds())
                .fold(
                    None,
                    |acc: Option<[Point3D; 2]>, [h_min, h_max]| match acc {
                        None => Some([h_min, h_max]),
                        Some([acc_min, acc_max]) => {
                            let new_min: DVec3 = DVec3::from(acc_min).min(h_min.into());
                            let new_max: DVec3 = DVec3::from(acc_max).max(h_max.into());
                            Some([new_min.into(), new_max.into()])
                        }
                    },
                )
            {
                if x[0].x == f64::MAX || x[1].x == f64::MIN {
                    return Err(JMdlError::BrushInvalid {
                        entity_idx: jmdl_entity_idx,
                    });
                }

                let mid_point = (x[0] + x[1]) / 2.;

                Ok(DVec3::new(mid_point.x, mid_point.y, mid_point.z))
            } else {
                Err(JMdlError::BrushInvalid {
                    entity_idx: jmdl_entity_idx,
                })
            }
        }
    } else {
        // simple path, the entity is a point entity
        Ok(map.entities[jmdl_entity_idx]
            .origin()
            .unwrap_or(DVec3::ZERO))
    }
}
