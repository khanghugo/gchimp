use std::{fs, path::Path};

use smd::{Smd, Triangle};

/// Duplicate every triangles with given texture name and rename texture name.
///
/// Mutate the input smd data.
pub fn duplicate_triangle(smd: &mut Smd, texture: &str, new_texture: &str) {
    let mut new_triangles: Vec<Triangle> = vec![];

    // Find triangles
    // Can't rayon this sad
    smd.triangles.as_ref().unwrap().iter().for_each(|tri| {
        // material might have .BMP at the end
        let mat = Path::new(&tri.material);

        if mat.file_stem().unwrap().to_str().unwrap() == texture {
            let mut our_tri = tri.clone();

            // strip .BMP just in case people forgot
            let new_mat = Path::new(&new_texture);

            // add .BMP back
            our_tri.material = format!(
                "{}.bmp",
                new_mat.file_stem().unwrap().to_str().unwrap().to_owned()
            );

            new_triangles.push(our_tri);
        }
    });

    // Add triangles
    new_triangles.iter().for_each(|tri| {
        smd.triangles.as_mut().unwrap().push(tri.clone());
    });
}

pub fn mass_duplicate_triangle(folder: &str, texture: &str, new_texture: &str) {
    let huh = fs::read_dir(folder).unwrap();
    let paths = huh.map(|path| path.unwrap().path());
    let smd_paths = paths.filter(|path| {
        path.is_file() && path.extension().is_some() && path.extension().unwrap() == "smd"
    });

    smd_paths.for_each(|path| {
        if let Ok(mut smd) = smd::Smd::from_file(path.to_str().unwrap()) {
            smd.duplicate_triangle(texture, new_texture);
            let _ = smd.write(path.to_str().unwrap());
        }
    });
}

pub trait DuplicateTriangleImpl {
    fn duplicate_triangle(&mut self, texture: &str, new_texture: &str) -> &mut Self;
}

impl DuplicateTriangleImpl for Smd {
    fn duplicate_triangle(&mut self, texture: &str, new_texture: &str) -> &mut Self {
        duplicate_triangle(self, texture, new_texture);
        self
    }
}
