use std::{ffi::CStr, path::Path};

use common::img_stuffs::{GoldSrcBmp, write_8bpp_to_file};
use mdl::Mdl;
use rayon::prelude::*;
use smd::Smd;

pub struct MdlDecompileResult {
    pub smds: Vec<(String, Smd)>,
    pub textures: Vec<(String, GoldSrcBmp)>,
}

pub fn mdl_decompile_native(path: &Path) -> Result<(), String> {
    let mdl = Mdl::open_from_file(path).map_err(|x| x.to_string())?;
    let MdlDecompileResult { smds, textures } = mdl_decompile(&mdl);

    // no need to handle errors
    // TODO: handle errors
    smds.par_iter().for_each(|(name, smd)| {
        let _ = smd.write(path.with_file_name(name).with_extension("smd"));
    });
    textures.par_iter().for_each(|(name, texture)| {
        let _ = write_8bpp_to_file(
            &texture.image,
            &texture.palette,
            texture.dimensions,
            path.with_file_name(name).with_extension("bmp"),
        );
    });

    Ok(())
}

pub fn mdl_decompile(mdl: &Mdl) -> MdlDecompileResult {
    let meshes = mdl::mdl_to_meshes(mdl);

    let arr_to_string = |x: &[u8]| {
        CStr::from_bytes_until_nul(x)
            .expect("model has empty texture name")
            .to_str()
            .unwrap()
            .to_string()
    };

    let textures: Vec<(String, GoldSrcBmp)> = mdl
        .textures
        .iter()
        .map(|x| {
            (
                arr_to_string(&x.header.name),
                GoldSrcBmp {
                    image: x.image.to_vec(),
                    palette: x.palette.to_vec(),
                    dimensions: x.dimensions(),
                },
            )
        })
        .collect();

    let nodes: Vec<_> = mdl
        .bones
        .iter()
        .enumerate()
        .map(|(bone_idx, bone)| smd::Node {
            id: bone_idx as i32,
            bone_name: arr_to_string(&bone.name),
            parent: bone.parent,
        })
        .collect();

    let smds: Vec<(String, Smd)> = meshes
        .into_iter()
        .map(|(name, mut mesh)| {
            let mut smd = Smd::new();

            // must flip mesh winding order
            mesh.iter_mut().for_each(|triangle| {
                triangle.flip_winding_order_mut();
                triangle.mdl_uv_to_normalized_smd_uv();
            });

            smd.triangles = mesh;
            smd.nodes = nodes.clone();

            // TODO do idle animation
            smd.skeleton = vec![smd::Skeleton::new_basic()];

            (name, smd)
        })
        .collect();

    MdlDecompileResult { smds, textures }
}
