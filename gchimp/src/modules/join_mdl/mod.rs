use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use common::setup_studio_model_transformations::setup_studio_model_transformations;
use glam::{DVec3, Mat3};
use map::Map;
use mdl::{Bodypart, Mdl, Texture, TrivertAffineTransformation};

use crate::{
    entity::GchimpInfo,
    modules::join_mdl::{
        entity::{
            JMDL_ATTR_MODEL_ENTITY, JMDL_ATTR_MODEL_TARGETS, JMDL_ATTR_OUTPUT, JMDL_ENTITY_NAME,
        },
        error::JMdlError,
    },
};

mod entity;
mod error;

pub fn join_model(map: &mut Map) -> Result<usize, JMdlError> {
    // verifies that there is gchimp_info
    // jmdl uses gchimp_info to find where the model is
    // will only search inside `basegame` and `basegame_downloads`
    let gchimp_info =
        GchimpInfo::from_map(&map).map_err(|op| JMdlError::GchimpInfo { source: op })?;

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
    let entities = map.get_entities_all(JMDL_ENTITY_NAME);

    let mut work_count = 0;

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

        // all other models with the same targetname
        let Some(target_model_entities) = map.entities[jmdl_entity_idx]
            .attributes
            .get(JMDL_ATTR_MODEL_TARGETS)
            .map(|s| {
                s.split_terminator(',')
                    .map(|entry| entry.trim())
                    .collect::<Vec<&str>>()
            })
        else {
            continue;
        };

        // list of entities with that targetname

        // TODO right now there is no check that the targetname must exists
        // so it is doing best effort to select models that exists
        let mut model_entities_indices = target_model_entities
            .iter()
            .filter_map(|x| map.get_entity_by_targetname(x))
            .collect::<Vec<usize>>();

        // model paths
        let model_paths = model_entities_indices
            .iter()
            .filter_map(|&idx| map.entities[idx].attributes.get("model"))
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

        // the model
        let mut combined_model = actually_join_models(&mdls, &translations, &rotations, &scales)?;

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
        model_entities_indices.sort();
        model_entities_indices.iter().rev().for_each(|idx| {
            map.entities.remove(*idx);
        });

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

        combined_model
            .write_to_file(output_absolute_path)
            .map_err(|op| JMdlError::MdlError { source: op })?;

        work_count += 1;
    }

    Ok(work_count)
}

fn actually_join_models(
    mdls: &[Mdl],
    translations: &[DVec3],
    rotations: &[DVec3],
    scales: &[f64],
) -> Result<Mdl, JMdlError> {
    let mut combined_mdl = Mdl::new_empty();

    // first, join all the textures
    // the same structure but uses number to index to avoid extra calculation
    // this is used for mesh to have the correct index
    let mut texture_processed_models_what: Vec<usize> = vec![];
    {
        let mut texture_combined: Vec<Texture> = vec![];

        // unique models that have had texture added
        // use a hashmap instead of hashset due to ergonomics
        let mut texture_processed_models: HashMap<String, usize> = HashMap::new();

        for mdl in mdls {
            let mdl_name = format!(
                "{}{}",
                String::from_utf8_lossy(mdl.header.name.as_slice()),
                mdl.triangle_count() // this is to make sure the name is totally good
            );

            let start_index = texture_processed_models.entry(mdl_name).or_insert_with(|| {
                // this is new model, must insert its texture to our list
                let start_index = texture_combined.len();

                for tex in &mdl.textures {
                    texture_combined.push(tex.clone());
                }

                start_index
            });

            // the difference betweenn this and the hashmap is that the hashmap length is always less than or equal to this vector
            texture_processed_models_what.push(*start_index);
        }

        if texture_combined.len() > mdl::MAX_TEXTURE {
            return Err(JMdlError::TooManyTextures {
                len: texture_combined.len(),
            });
        }

        combined_mdl.textures = texture_combined;
        // combined_mdl.textures = mdls[0].textures.clone(); // debug
    }

    // next, join all the bodies
    {
        let mut bodypart_combined: Vec<Bodypart> = vec![];

        for (idx, mdl) in mdls.iter().enumerate() {
            let texture_start_index = texture_processed_models_what[idx];

            let mut owned_bodyparts = mdl.bodyparts.clone();

            // need to apply the model idle sequence to get the "idle" geometry

            // TODO this mean it is possible to bake a model with multiple bones
            let (bone_pos, bone_rot) = setup_studio_model_transformations(mdl)
            [0] // sequence
            [0] // blend
            [0] // frame
            [0].clone() // bone 0
            ;

            // must use matrix to avoid implicit rotation order
            let cg_mat: cgmath::Matrix3<f32> = bone_rot.into();
            let mat_array: [[f32; 3]; 3] = cg_mat.into();
            let bone_pos_glam = glam::vec3(bone_pos.x, bone_pos.y, bone_pos.z);

            owned_bodyparts.iter_mut().for_each(|bodypart| {
                // reduce the model count to just 1
                bodypart.models = vec![bodypart.models[0].clone()];

                // point to the new texture bunch
                bodypart.models[0].meshes.iter_mut().for_each(|mesh| {
                    mesh.header.skin_ref += texture_start_index as i32;
                });

                // affine transformation

                // TODO
                // again, this assumes static model with ONE bone
                // if the model has more than 1 bone, this would easily bork the mesh
                // but everything is fine
                bodypart.models[0].meshes.iter_mut().for_each(|mesh| {
                    mesh.triangles.iter_mut().for_each(|triangles| {
                        // local/bone transformation
                        triangles.transform_mat3(Mat3::from_cols_array_2d(&mat_array));
                        triangles.translate(bone_pos_glam);

                        // world transformation
                        triangles.scale(scales[idx] as f32);
                        triangles.rotate(rotations[idx].as_vec3());
                        triangles.translate(translations[idx].as_vec3());
                    });
                });

                // force the vertex and normal bone to 0 cuz we have 1 bone
                bodypart.models[0].normal_info.fill(0);
                bodypart.models[0].vertex_info.fill(0);
            });

            bodypart_combined.append(&mut owned_bodyparts);
        }

        combined_mdl.bodyparts = bodypart_combined;
        // combined_mdl.bodyparts = mdls[0].bodyparts.clone(); // debug
    }

    // should be good???
    // so far i guess

    Ok(combined_mdl)
}

#[cfg(test)]
mod test {
    use mdl::Mdl;

    use crate::modules::join_mdl::actually_join_models;

    #[test]
    #[allow(unused)]
    fn run() {
        let bytes = include_bytes!("/home/khang/gchimp/mdl/src/tests/static_tree.mdl");

        let mdl1 = Mdl::open_from_bytes(bytes).unwrap();
        let mdl2 = mdl1.clone();
        let mdl3 = mdl1.clone();

        let transformations = vec![
            [0., 0., 0.].into(),
            [64., 0., 0.].into(),
            [-64., 0., 0.].into(),
        ];

        let rotations = vec![[0., 0., 0.].into(); 3];
        let scales = vec![1.; 3];

        let mut combined_mdl = actually_join_models(
            vec![mdl1, mdl2, mdl3].as_ref(),
            &transformations,
            &rotations,
            &scales,
        )
        .unwrap();

        combined_mdl.rebuild_data_for_export();

        // must set name, otherwise HLAM rejects
        combined_mdl.set_name("hello_world.mdl");

        println!("{:?}", combined_mdl.header.name);

        let out_bytes = combined_mdl.write_to_bytes();

        combined_mdl
            .write_to_file("/home/khang/gchimp/mdl/src/tests/static_tree_combined.mdl")
            .unwrap();
    }
}
