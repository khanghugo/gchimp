mod errors;
mod tests;
mod types;

pub use errors::*;
pub use types::*;

use std::collections::{HashMap, HashSet};

use mdl::Mdl;

impl StudioMdl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_model_name(&mut self, s: impl Into<String>) -> &mut Self {
        self.name = s.into();

        self
    }

    pub fn add_bodypart(&mut self, mesh: Mesh) -> &mut Self {
        self.meshes.push(mesh);

        self
    }

    pub fn add_texture(&mut self, texture: impl Into<Texture>) -> &mut Self {
        self.textures.push(texture.into());

        self
    }

    /// Adds triangle to the first bodypart
    pub fn add_triangle(&mut self, triangle: impl Into<smd::Triangle>) -> &mut Self {
        if self.meshes.is_empty() {
            self.meshes.push(Mesh::default());
        }

        self.meshes[0].mesh.push(triangle.into());

        self
    }

    fn list_used_materials(&self) -> HashSet<&str> {
        self.meshes
            .iter()
            .flat_map(|mesh| {
                mesh.mesh
                    .iter()
                    .map(|tri| tri.material.as_str())
                    .collect::<HashSet<&str>>()
            })
            .collect()
    }

    fn list_available_materials(&self) -> HashSet<&str> {
        self.textures
            .iter()
            .map(|texture| texture.name.as_str())
            .collect()
    }

    fn check_if_all_materials_are_listed(&self) -> Result<(), StudioMdlError> {
        let available = self.list_available_materials();
        let used = self.list_used_materials();
        let missing: Vec<&&str> = used.difference(&available).collect();

        if !missing.is_empty() {
            return Err(StudioMdlError::MissingTextures {
                textures: missing.into_iter().map(|x| x.to_string()).collect(),
            });
        }

        Ok(())
    }

    pub fn compile(mut self) -> Result<Mdl, StudioMdlError> {
        // do some checks first
        self.check_if_all_materials_are_listed()?;

        let mut mdl = Mdl::new_empty();

        // add textures
        // TODO: this is actually double work considering that `mdl` does the look up again
        let mut texture_lookup: HashMap<String, usize> = HashMap::new();

        self.textures
            .into_iter()
            .enumerate()
            .for_each(|(texture_idx, texture)| {
                let mut new_texture = mdl::Texture::new_texture(
                    &texture.name,
                    texture.dimensions,
                    texture.image,
                    texture.palette,
                    texture.flag,
                );

                new_texture.header.index = texture_idx as i32;

                mdl.textures.push(new_texture);
                texture_lookup.insert(texture.name, texture_idx);
            });

        // add meshes
        self.meshes.iter_mut().for_each(|mesh| {
            let new_bodypart = mdl::Bodypart {
                header: {
                    let mut header = mdl::BodypartHeader::default();

                    header.set_name(&mesh.name);
                    header.base = 1; // base = 1 or crash

                    header
                },
                models: {
                    let mut new_model = mdl::Model::default();
                    new_model.set_name(&mesh.name);

                    mesh.reverse_winding_order(); // must reverse order
                    mesh.fix_uv(); // y coordinate is different

                    new_model.agnostic_mesh = Some(mesh.mesh.clone());

                    vec![new_model]
                },
            };

            mdl.bodyparts.push(new_bodypart);
        });

        // other settings
        mdl.set_name(&self.name);

        mdl.rebuild_data_for_export(); // this does lots of other stuffs, should use it

        Ok(mdl)
    }
}
