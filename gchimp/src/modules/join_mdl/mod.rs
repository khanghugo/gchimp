use std::path::{Path, PathBuf};

use glam::DVec3;
use map::Map;
use mdl::Mdl;

use crate::{
    gchimp_info::{GchimpInfo, GchimpInfoOption},
    modules::join_mdl::{
        entity::{JMDL_ATTR_MODEL_ENTITY, JMDL_ATTR_OUTPUT, JMDL_ENTITY_NAME},
        error::JMdlError,
    },
    utils::mdl_stuffs::{JoinMdlsParameters, join_mdls_with_affine_transformation},
};

mod entity;
mod error;

pub fn join_model(map: &mut Map) -> Result<usize, JMdlError> {
    // verifies that there is gchimp_info
    // jmdl uses gchimp_info to find where the model is
    // will only search inside `basegame` and `basegame_downloads`
    let gchimp_info = GchimpInfo::from_map(&map)?;

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

    // does this over all gchimp_jmdl
    let entities = map.get_entities_by_classname_all(JMDL_ENTITY_NAME);

    let mut work_count = 0;

    // delete entities later when all the entities are processed
    // otherwise, the entity index no longer works
    let mut entities_to_delete = vec![];

    for jmdl_entity_idx in entities {
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

        // list of model entities that point to our gchimp_jmdl
        // this just has better ergonomics
        // if gchimp_jmdl contains the model targetname instead, you have to
        // name the model and then insert it to the gchimp_mdl
        let jmdl_targetname = map.entities[jmdl_entity_idx]
            .targetname()
            .cloned()
            .ok_or(JMdlError::NoTargetName)?;

        let mut model_entities_indices = map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, ent)| ent.target().map_or(false, |t| t == &jmdl_targetname))
            .map(|(idx, _)| idx)
            .collect::<Vec<usize>>();

        // skip if nothing, easy
        if model_entities_indices.is_empty() {
            continue;
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
        let jmdl_entity_origin = map.entities[jmdl_entity_idx]
            .origin()
            .unwrap_or(DVec3::ZERO);
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
