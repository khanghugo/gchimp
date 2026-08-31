use std::array::from_fn;

use common::img_stuffs::GoldSrcBmp;
use mdl::PALETTE_COUNT;
use smd::Triangle;

#[derive(Debug, Clone, Default)]
pub struct StudioMdl {
    pub name: String,
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,

    // internal variables
    pub(crate) bodypart_index: usize,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub name: String,
    pub mesh: Vec<smd::Triangle>,
}

impl Mesh {
    pub fn reverse_winding_order(&mut self) {
        self.mesh.iter_mut().for_each(|tri| {
            tri.vertices.reverse();
        });
    }

    pub fn normalized_smd_uv_to_mdl_uv(&mut self) {
        self.mesh.iter_mut().for_each(|tri| {
            tri.vertices
                .iter_mut()
                .for_each(|vert| vert.uv = (vert.uv.x, 1.0 - vert.uv.y).into());
        });
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            name: "default".into(),
            mesh: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub name: String,
    pub dimensions: (u32, u32),
    pub image: Vec<u8>,
    pub palette: [[u8; 3]; PALETTE_COUNT],
    pub flag: mdl::TextureFlag,
}

impl<S>
    From<(
        S,                        // Texture name
        (u32, u32),               // Dimensions
        Vec<u8>,                  // Image data
        [[u8; 3]; PALETTE_COUNT], // Palette
        mdl::TextureFlag,         // Flag
    )> for Texture
where
    S: Into<String> + AsRef<str>,
{
    fn from(
        value: (
            S,
            (u32, u32),
            Vec<u8>,
            [[u8; 3]; PALETTE_COUNT],
            mdl::TextureFlag,
        ),
    ) -> Self {
        Texture {
            name: value.0.into(),
            dimensions: value.1,
            image: value.2,
            palette: value.3,
            flag: value.4,
        }
    }
}

impl<S> From<(S, GoldSrcBmp, mdl::TextureFlag)> for Texture
where
    S: Into<String> + AsRef<str>,
{
    fn from(mut value: (S, GoldSrcBmp, mdl::TextureFlag)) -> Self {
        value.1.pad_palette();

        Texture {
            name: value.0.into(),
            dimensions: value.1.dimensions,
            image: value.1.image,
            palette: from_fn(|i| value.1.palette[i]),
            flag: value.2,
        }
    }
}

impl From<(String, Vec<Triangle>)> for Mesh {
    fn from(value: (String, Vec<Triangle>)) -> Self {
        Self {
            name: value.0,
            mesh: value.1,
        }
    }
}
