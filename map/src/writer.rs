use std::{
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::Map;

impl Map {
    pub fn write(&self, path: impl AsRef<Path> + Into<PathBuf>) -> io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut file = BufWriter::new(file);

        if let Some(tb_header) = &self.tb_header {
            for s in tb_header {
                file.write_all("//".as_bytes())?;
                file.write_all(s.as_bytes())?;
                file.write_all("\n".as_bytes())?;
            }
        }

        for (entity_index, entities) in self.entities.iter().enumerate() {
            file.write_all(format!("// entity {}\n", entity_index).as_bytes())?;

            file.write_all("{\n".as_bytes())?;

            for (key, value) in &entities.attributes {
                file.write_all(format!("\"{}\" \"{}\"\n", key, value).as_bytes())?;
            }

            if let Some(brushes) = &entities.brushes {
                for (brush_entity, brush) in brushes.iter().enumerate() {
                    file.write_all(format!("// brush {}\n", brush_entity).as_bytes())?;
                    file.write_all("{\n".as_bytes())?;

                    for plane in &brush.planes {
                        file.write_all(format!("( {} {} {} ) ( {} {} {} ) ( {} {} {} ) {} [ {} {} {} {} ] [ {} {} {} {} ] {} {} {}\n", 
                    plane.p1.x,plane.p1.y,plane.p1.z,
                    plane.p2.x,plane.p2.y,plane.p2.z,
                    plane.p3.x,plane.p3.y,plane.p3.z,
                    plane.texture_name.get_string(),
                    plane.u.x,plane.u.y,plane.u.z,plane.u.w,
                    plane.v.x,plane.v.y,plane.v.z,plane.v.w,
                    plane.rotation, plane.u_scale, plane.v_scale,

                ).as_bytes())?;
                    }
                    file.write_all("}\n".as_bytes())?;
                }
            }

            file.write_all("}\n".as_bytes())?;
        }

        file.flush()?;

        Ok(())
    }
}
