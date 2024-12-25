mod nom_helpers;
mod parser;
mod types;

pub use types::Mdl;
pub use types::*;

#[cfg(test)]
mod test {
    use std::mem;

    use crate::{
        types::{Header, SequenceDescription, TextureHeader},
        BodypartHeader, MeshHeader, ModelHeader, TrivertHeader,
    };

    #[test]
    fn assert_struct_size() {
        assert_eq!(mem::size_of::<Header>(), 244);
        assert_eq!(mem::size_of::<SequenceDescription>(), 176);
        assert_eq!(mem::size_of::<TextureHeader>(), 80);
        assert_eq!(mem::size_of::<BodypartHeader>(), 76);
        assert_eq!(mem::size_of::<ModelHeader>(), 112);
        assert_eq!(mem::size_of::<MeshHeader>(), 20);
        assert_eq!(mem::size_of::<TrivertHeader>(), 8);
    }
}
