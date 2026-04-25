use std::{any::Any, collections::HashMap, path::Path, process::Output, str::from_utf8};

use common::setup_studio_model_transformations::setup_studio_model_transformations;
use eyre::eyre;
use glam::{DVec3, Mat3};
use mdl::{Bodypart, Mdl, Texture, TrivertAffineTransformation, error::MdlError};

use super::constants::STUDIOMDL_ERROR_PATTERN;

pub fn handle_studiomdl_output(
    res: Result<Result<Output, eyre::Report>, Box<dyn Any + Send>>,
    _path: Option<&Path>,
) -> eyre::Result<()> {
    match res {
        Ok(res) => {
            let output = res.unwrap();
            let stdout = from_utf8(&output.stdout).unwrap();

            let maybe_err = stdout.find(STUDIOMDL_ERROR_PATTERN);

            if let Some(err_index) = maybe_err {
                let err = stdout[err_index + STUDIOMDL_ERROR_PATTERN.len()..].to_string();

                // this message makes it too long and too redundant
                // let err_str = if let Some(path) = path {
                //     format!("cannot compile {}: {}", path.display(), err.trim())
                // } else {
                //     format!("cannot compile mdl: {}", err.trim())
                // };

                let err_str = err.trim().to_owned();

                return Err(eyre!(err_str));
            }

            Ok(())
        }
        Err(_) => {
            let err_str = "No idea what happens with running studiomdl. Probably just a dream.";

            Err(eyre!(err_str))
        }
    }
}

pub struct JoinMdlsParameters {
    pub translations: Vec<DVec3>,
    pub rotations: Vec<DVec3>,
    pub scales: Vec<f64>,
    pub sequences: Vec<u32>,
}

pub fn join_mdls_with_affine_transformation(
    mdls: &[Mdl],
    parameter: JoinMdlsParameters,
) -> Result<Mdl, MdlError> {
    let mut combined_mdl = Mdl::new_empty();

    let JoinMdlsParameters {
        translations,
        rotations,
        scales,
        sequences,
    } = parameter;

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
            return Err(MdlError::TooManyTextures {
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
            let frame0 = setup_studio_model_transformations(mdl)
            [sequences[idx] as usize] // sequence
            [0] // blend
            [0].clone() // frame
            ;

            let frame0_mappable = frame0
                .into_iter()
                .map(|(bone_pos, bone_rot)| {
                    // must use matrix to avoid implicit rotation order
                    let cg_mat: cgmath::Matrix3<f32> = bone_rot.into();
                    let mat_array: [[f32; 3]; 3] = cg_mat.into();
                    let bone_pos_glam = glam::vec3(bone_pos.x, bone_pos.y, bone_pos.z);

                    (mat_array, bone_pos_glam)
                })
                .collect::<Vec<_>>();

            owned_bodyparts.iter_mut().for_each(|bodypart| {
                // reduce the model count to just 1
                bodypart.models = vec![bodypart.models[0].clone()];

                // point to the new texture bunch
                bodypart.models[0].meshes.iter_mut().for_each(|mesh| {
                    mesh.header.skin_ref += texture_start_index as i32;
                });

                // affine transformation
                bodypart.models.iter_mut().for_each(|model| {
                    model.meshes.iter_mut().for_each(|mesh| {
                        mesh.triangles.iter_mut().for_each(|triangles| {
                            // local/bone transformation
                            // should translate per vertex, not triangle
                            triangles.get_triverts_mut().iter_mut().for_each(|trivert| {
                                let vertex_info = &model.vertex_info;
                                let bone_idx = vertex_info[trivert.header.vert_index as usize];
                                let transform_mat =
                                    Mat3::from_cols_array_2d(&frame0_mappable[bone_idx as usize].0);
                                let bone_pos_glam = frame0_mappable[bone_idx as usize].1;

                                trivert.transform_mat3(transform_mat);
                                trivert.translate(bone_pos_glam);
                            });

                            // world transformation
                            triangles.scale(scales[idx] as f32);
                            triangles.rotate(rotations[idx].as_vec3());
                            triangles.translate(translations[idx].as_vec3());
                        });
                    })
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

    use crate::utils::mdl_stuffs::{JoinMdlsParameters, join_mdls_with_affine_transformation};

    #[test]
    #[allow(unused)]
    fn run() {
        let bytes = include_bytes!("/home/khang/gchimp/mdl/src/tests/static_tree.mdl");

        let mdl1 = Mdl::open_from_bytes(bytes).unwrap();
        let mdl2 = mdl1.clone();
        let mdl3 = mdl1.clone();

        let translations = vec![
            [0., 0., 0.].into(),
            [64., 0., 0.].into(),
            [-64., 0., 0.].into(),
        ];

        let rotations = vec![[0., 0., 0.].into(); 3];
        let scales = vec![1.; 3];

        let mut combined_mdl = join_mdls_with_affine_transformation(
            vec![mdl1, mdl2, mdl3].as_ref(),
            JoinMdlsParameters {
                translations,
                rotations,
                scales,
                sequences: vec![0; 3],
            },
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
