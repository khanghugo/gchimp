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
        BodypartHeader, Bone, BoneController, Hitbox, Mdl, MeshHeader, ModelHeader, SequenceGroup,
        TrivertHeader,
        types::{Header, SequenceHeader, TextureHeader},
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
        let mdl = Mdl::open_from_bytes(bytes).unwrap();
    }

    #[test]
    fn parse_usp() {
        let bytes = include_bytes!("./tests/v_usp.mdl");
        let mdl = Mdl::open_from_bytes(bytes).unwrap();
    }
}
