use mdl::PALETTE_COUNT;

#[derive(Debug, Clone)]
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

impl<S> Into<Texture>
    for (
        S,
        (u32, u32),
        Vec<u8>,
        [[u8; 3]; PALETTE_COUNT],
        mdl::TextureFlag,
    )
where
    S: Into<String> + AsRef<str>,
{
    fn into(self) -> Texture {
        Texture {
            name: self.0.into(),
            dimensions: self.1,
            image: self.2,
            palette: self.3,
            flag: self.4,
        }
    }
}

impl Default for StudioMdl {
    fn default() -> Self {
        Self {
            name: Default::default(),
            meshes: Default::default(),
            textures: Default::default(),
        }
    }
}
