use smd::Smd;

use super::constants::MAX_SMD_TRIANGLE;

pub fn source_smd_to_goldsrc_smd(smd: &Smd) -> Vec<Smd> {
    let mut res: Vec<Smd> = vec![];

    // No triangles means no need to split so just use the original
    if smd.triangles.is_none() {
        res.push(smd.clone());

        return res;
    }

    let old_triangles = smd.triangles.as_ref().unwrap();

    let needed_smd = old_triangles.len() / MAX_SMD_TRIANGLE + 1;

    (0..needed_smd).for_each(|index| {
        let mut new_smd = Smd {
            nodes: smd.nodes.clone(),
            skeleton: smd.skeleton.clone(),
            triangles: Some(
                old_triangles
                    .chunks(MAX_SMD_TRIANGLE)
                    .nth(index)
                    .unwrap()
                    .to_vec(),
            ),
            ..Default::default()
        };

        // fix the triangles
        new_smd
            .triangles
            .as_mut()
            .unwrap()
            .iter_mut()
            .for_each(|tri| {
                // remove the Source part
                tri.vertices
                    .iter_mut()
                    .for_each(|vertex| vertex.source = None);

                // make the texture name no space
                tri.material = tri.material.replace(" ", "_");

                // make the texture name lower case
                tri.material = tri.material.to_lowercase();

                // goldsrc models need .bmp in the name
                if !tri.material.ends_with(".bmp") {
                    tri.material += ".bmp";
                }
            });

        res.push(new_smd);
    });

    res
}
