use bitflags::bitflags;

// make sure data here match gchimp.fgd and vice versa
pub const MAP2MDL_ENTITY_NAME: &str = "gchimp_map2mdl";

pub const MAP2MDL_ATTR_OUTPUT: &str = "output";

pub const MAP2MDL_ATTR_MODEL_ENTITY: &str = "model_entity";

pub const MAP2MDL_ATTR_CLIPTYPE: &str = "cliptype";

pub const MAP2MDL_ATTR_TARGET_ORIGIN: &str = "target_origin";
pub const MAP2MDL_ATTR_TARGET_ORIGIN_ENTITY: &str = "info_target";

pub const MAP2MDL_ATTR_OPTIONS: &str = "options";

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Map2MdlEntityOptions: u32 {
        const FlatShade = 1 << 0;
        /// Containing the original brush and celshade
        const WithCelShade = 1 << 1;
        /// Turning the brush into just celshade
        const AsCelShade = 1 << 2;
        /// Reverses all normals in the model. This is mainly for reflection scenes.
        const ReverseNormals = 1 << 3;
    }
}

pub const MAP2MDL_ATTR_CELSHADE_COLOR: &str = "celshade_color";
pub const MAP2MDL_ATTR_CELSHADE_DISTANCE: &str = "celshade_distance";
