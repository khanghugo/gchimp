//! Based on
//!
//! https://github.com/malortie/assimp/wiki/MDL:-Half-Life-1-file-format
//!
pub mod error;
mod nom_helpers;
mod parser;
mod types;
mod writer;

pub use types::Mdl;
pub use types::*;

#[cfg(test)]
mod test {
    use std::mem;

    use crate::{
        types::{Header, SequenceHeader, TextureHeader},
        BodypartHeader, Bone, BoneController, Hitbox, Mdl, MeshHeader, ModelHeader, SequenceGroup,
        Trivert, TrivertHeader,
    };

    #[test]
    fn assert_struct_size() {
        assert_eq!(mem::size_of::<Header>(), 244);
        assert_eq!(mem::size_of::<SequenceHeader>(), 176);
        assert_eq!(mem::size_of::<TextureHeader>(), 80);
        assert_eq!(mem::size_of::<BodypartHeader>(), 76);
        assert_eq!(mem::size_of::<ModelHeader>(), 112);
        assert_eq!(mem::size_of::<MeshHeader>(), 20);
        assert_eq!(mem::size_of::<TrivertHeader>(), 8);
        assert_eq!(mem::size_of::<Bone>(), 112);
        assert_eq!(mem::size_of::<BoneController>(), 24);
        assert_eq!(mem::size_of::<Hitbox>(), 32);
        assert_eq!(mem::size_of::<SequenceGroup>(), 104);
    }

    #[test]
    /// Model with external texture
    fn parse_orange() {
        let bytes = include_bytes!("./tests/orange.mdl");
        let mdl = Mdl::open_from_bytes(bytes).unwrap();

        assert_eq!(mdl.textures.len(), 0);
    }

    #[test]
    fn parse_chick() {
        let bytes = include_bytes!("./tests/chick.mdl");
        let _mdl = Mdl::open_from_bytes(bytes).unwrap();
    }

    #[test]
    fn parse_usp() {
        let bytes = include_bytes!("./tests/v_usp.mdl");
        let _mdl = Mdl::open_from_bytes(bytes).unwrap();
    }

    #[test]
    fn parse_write_parse_static_tree() {
        let bytes = include_bytes!("./tests/static_tree.mdl");

        let mdl = Mdl::open_from_bytes(bytes).unwrap();

        let bytes2 = mdl.write_to_bytes();
        let mdl2 = Mdl::open_from_bytes(&bytes2).unwrap();

        println!("{:?}", mdl.bodyparts[0].models[0].meshes[0].triangles.len());
        println!(
            "{:?}",
            mdl2.bodyparts[0].models[0].meshes[0].triangles.len()
        );

        // no animation
        assert_eq!(mdl.sequences[0].anim_blends, mdl2.sequences[0].anim_blends);

        // check triangles
        let contains = |y: &Vec<Trivert>| {
            mdl2.bodyparts[0].models[0].meshes[0]
                .triangles
                .iter()
                .any(|x| x.get_triverts() == y)
        };

        mdl.bodyparts[0].models[0].meshes[0]
            .triangles
            .iter()
            .for_each(|triangle| assert!(contains(triangle.get_triverts())));

        // do it again becuase i might be crazy
        mdl.bodyparts[0].models[0].meshes[0]
            .triangles
            .iter()
            .zip(mdl2.bodyparts[0].models[0].meshes[0].triangles.iter())
            .for_each(|(t1, t2)| match t1 {
                crate::MeshTriangles::Strip(triverts1) => {
                    let crate::MeshTriangles::Strip(triverts2) = t2 else {
                        panic!()
                    };

                    assert_eq!(triverts1.len(), triverts2.len());

                    triverts1.iter().zip(triverts2.iter()).for_each(|(t1, t2)| {
                        assert_eq!(t1.normal, t2.normal);
                        assert_eq!(t1.vertex, t2.vertex);
                    });
                }
                crate::MeshTriangles::Fan(triverts1) => {
                    let crate::MeshTriangles::Fan(triverts2) = t2 else {
                        panic!()
                    };

                    assert_eq!(triverts1.len(), triverts2.len());

                    triverts1.iter().zip(triverts2.iter()).for_each(|(t1, t2)| {
                        assert_eq!(t1.normal, t2.normal);
                        assert_eq!(t1.vertex, t2.vertex);
                    });
                }
            });

        // check bones
        assert_eq!(mdl.bones, mdl2.bones);

        // check textures
        assert_eq!(mdl.textures, mdl2.textures);

        // check transitions
        assert_eq!(mdl.transitions, mdl2.transitions);

        // check skin families
        assert_eq!(mdl.skin_families, mdl2.skin_families);

        // check bone controllers
        assert_eq!(mdl.bone_controllers, mdl2.bone_controllers);

        // check hitbox
        assert_eq!(mdl.hitboxes, mdl2.hitboxes);

        // write the file
        mdl.write_to_file("/home/khang/gchimp/mdl/src/tests/static_tree_parse_write.mdl")
            .unwrap();
    }
}
