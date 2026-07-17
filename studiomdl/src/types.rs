use std::array::from_fn;

use common::img_stuffs::GoldSrcBmp;
use mdl::PALETTE_COUNT;

#[derive(Debug, Clone, Default)]
pub struct StudioMdl {
    pub name: String,
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
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

    pub fn fix_uv(&mut self) {
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
        S,
        (u32, u32),
        Vec<u8>,
        [[u8; 3]; PALETTE_COUNT],
        mdl::TextureFlag,
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
