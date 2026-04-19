use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use glam::DVec3;
use map::Map;
use mdl::{Bodypart, Mdl, Texture, TrivertAffineTransformation};

use crate::{
    entity::GchimpInfo,
    modules::join_mdl::{
        entity::{JMDL_ATTR_MODEL_TARGETS, JMDL_ENTITY_NAME},
        error::JMdlError,
    },
};

mod entity;
mod error;

pub fn join_model(map: &Map) -> Result<Map, JMdlError> {
    let mut map = map.clone();

    // verifies that there is gchimp_info
    // jmdl uses gchimp_info to find where the model is
    // will only search inside `basegame` and `basegame_downloads`
    let gchimp_info =
        GchimpInfo::from_map(&map).map_err(|op| JMdlError::GchimpInfo { source: op })?;

    let game_dirs = if let Some(stripped) = gchimp_info.gamedir().strip_suffix("_downloads") {
        vec![stripped.to_owned(), format!("{}_downloads", stripped)]
    } else {
        let gamedir = gchimp_info.gamedir();

        vec![gamedir.to_owned(), format!("{}_downloads", gamedir)]
    };

    // verify that the gamedir exists oor just not
    let model_prefixes = game_dirs
        .iter()
        .map(|x| Path::new(gchimp_info.hl_path()).join(x))
        .filter(|x| x.exists())
        .collect::<Vec<PathBuf>>();

    // does this over all gchimp_jmdl
    let entities = map.get_entities_all(JMDL_ENTITY_NAME);

    for entity_idx in entities {
        // current gchimp_jmdl
        let entity = &map.entities[entity_idx];

        // all other models with the same targetname
        let Some(target_model_entities) = entity.attributes.get(JMDL_ATTR_MODEL_TARGETS).map(|s| {
            s.split_terminator(',')
                .map(|entry| entry.trim())
                .collect::<Vec<&str>>()
        }) else {
            continue;
        };

        // list of entities with that targetname

        // TODO right now there is no check that the targetname must exists
        // so it is doing best effort to select models that exists
        let model_entities = target_model_entities
            .iter()
            .filter_map(|x| map.get_entity_by_targetname(x))
            .collect::<Vec<usize>>();

        // model paths
        let model_paths = model_entities
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
        let mut mdls = model_full_paths
            .iter()
            .filter_map(|path| match Mdl::open_from_file(path) {
                Ok(x) => Some(x),
                Err(x) => {
                    println!("Failed to open {:?} {}", path, x);
                    None
                }
            })
            .collect::<Vec<Mdl>>();

        // build data for all models
        mdls.iter_mut()
            .for_each(|mdl| mdl.rebuild_data_for_export());
    }

    Ok(map)
}

fn actually_join_models(
    mdls: &[Mdl],
    translations: &[DVec3],
    rotations: &[DVec3],
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
    }

    // next, join all the bodies
    {
        let mut bodypart_combined: Vec<Bodypart> = vec![];

        for (idx, mdl) in mdls.iter().enumerate() {
            let texture_start_index = texture_processed_models_what[idx];

            let mut owned_bodyparts = mdl.bodyparts.clone();

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
                        triangles.rotate(rotations[idx].as_vec3());
                        triangles.translate(translations[idx].as_vec3());
                    });
                });
            });

            bodypart_combined.append(&mut owned_bodyparts);
        }

        combined_mdl.bodyparts = bodypart_combined;
    }

    // should be good???

    Ok(combined_mdl)
}

#[cfg(test)]
mod test {
    use mdl::Mdl;

    use crate::modules::join_mdl::actually_join_models;

    #[test]
    fn run() {
        let bytes = include_bytes!("/home/khang/gchimp/mdl/src/tests/static_tree.mdl");

        let mdl1 = Mdl::open_from_bytes(bytes).unwrap();
        let mdl2 = mdl1.clone();
        let mdl3 = mdl1.clone();

        let transformations = vec![
            [0., 0., 0.].into(),
            [128., 0., 0.].into(),
            [-128., 0., 0.].into(),
        ];

        let rotations = vec![[0., 0., 0.].into(); 3];

        let mut res = actually_join_models(
            vec![mdl1, mdl2, mdl3].as_ref(),
            &transformations,
            &rotations,
        )
        .unwrap();

        res.rebuild_data_for_export();

        // must set name, otherwise HLAM rejects
        res.set_name("hello_world.mdl");

        let out_bytes = res.write_to_bytes();

        let mdl_out = Mdl::open_from_bytes(&out_bytes).unwrap();
        println!("{:?}", mdl_out.header);

        res.write_to_file("/home/khang/gchimp/mdl/src/tests/static_tree_combined.mdl")
            .unwrap();
    }
}
