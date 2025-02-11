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
        BodypartHeader, Bone, BoneController, Hitbox, MeshHeader, ModelHeader, SequenceGroup,
        TrivertHeader,
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
}
