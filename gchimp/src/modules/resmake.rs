use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use bsp::Bsp;
use chrono::Local;
use wad::types::Wad;

use crate::{
    err,
    utils::constants::{MODEL_ENTITIES, SOUND_ENTITIES, SPRITE_ENTITIES},
};

pub struct ResMakeOptions {
    /// Wheter to includes external WAD inside .res
    wad_check: bool,
}

impl Default for ResMakeOptions {
    fn default() -> Self {
        Self { wad_check: false }
    }
}

// need to be a vector so we can sort it by wad file name
type WadTable = Vec<(String, HashSet<String>)>;

pub struct ResMake {
    bsp_file: Option<PathBuf>,
    // If this is set to gamemod folder, it will do mass ResMake
    root_folder: Option<PathBuf>,
    options: ResMakeOptions,
}

impl Default for ResMake {
    fn default() -> Self {
        Self {
            bsp_file: Default::default(),
            root_folder: Default::default(),
            options: Default::default(),
        }
    }
}

impl ResMake {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bsp_file(&mut self, path: impl AsRef<Path> + Into<PathBuf>) -> &mut Self {
        self.bsp_file = Some(path.into());

        self
    }

    pub fn root_folder(&mut self, path: impl AsRef<Path> + Into<PathBuf>) -> &mut Self {
        self.root_folder = Some(path.into());

        self
    }

    pub fn wad_check(&mut self, v: bool) -> &mut Self {
        self.options.wad_check = v;

        self
    }

    fn check_bsp_file(&self) -> eyre::Result<()> {
        let Some(path) = self.bsp_file.as_ref() else {
            return err!("bsp_file is not set");
        };

        if !path.exists() {
            return err!("bsp file `{}` does not exist", path.display());
        }

        if !path.is_file() {
            return err!("bsp file `{}` is not a file", path.display());
        }

        if let Some(ext) = path.extension() {
            if ext != "bsp" {
                return err!("bsp file `{}` is not a bsp file", path.display());
            }
        } else {
            return err!("bsp file `{}` does not have any extension", path.display());
        }

        Ok(())
    }

    fn check_bsp_file_parent(&self) -> eyre::Result<PathBuf> {
        let Some(path) = self.bsp_file.as_ref() else {
            return err!("bsp_file is not set");
        };

        // now we have a /path/to/gamemod/maps/bsp.bsp
        // need to assert that we are inside a gamemod
        if let Some(maps_folder) = path.parent() {
            if let Some(map_folder_name) = maps_folder.file_name() {
                if map_folder_name != "maps" {
                    return err!("bsp file is not inside `maps` folder");
                }
            } else {
                return err!("bsp file is inside a folder without name");
            }

            if maps_folder.parent().is_none() {
                return err!(
                    "`maps` folder is not inside a gamemod folder such as `cstrike` or `valve`"
                );
            }
        }

        Ok(path
            .parent()
            .and_then(|path| path.parent())
            .unwrap()
            .to_path_buf())
    }

    fn generate_wad_table(&self) -> eyre::Result<WadTable> {
        let root_folder = if let Some(_) = &self.bsp_file {
            self.check_bsp_file()?;
            self.check_bsp_file_parent()?
        } else if let Some(root_folder) = &self.root_folder {
            root_folder.to_path_buf()
        } else {
            return err!("no folder set");
        };

        let res = generate_wad_table(&root_folder);

        res
    }

    pub fn _get_resmake_single_bsp_string(&self) -> eyre::Result<String> {
        self.check_bsp_file()?;
        self.check_bsp_file_parent()?;

        let wad_table = if self.options.wad_check {
            Some(self.generate_wad_table()?)
        } else {
            None
        };

        let bsp = Bsp::from_file(self.bsp_file.as_ref().unwrap())?;

        resmake_single_bsp(
            &bsp,
            self.bsp_file.as_ref().unwrap(),
            wad_table.as_ref(),
            &self.options,
        )
    }

    pub fn single_bsp(&self) -> eyre::Result<()> {
        let res_string = self._get_resmake_single_bsp_string()?;

        let Some(bsp_path) = self.bsp_file.as_ref() else {
            return err!("no bsp path given");
        };

        let out_path = bsp_path.with_extension("res");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(out_path)?;

        file.write_all(res_string.as_bytes())?;
        file.flush()?;

        Ok(())
    }
}

fn need_external_wad(bsp: &Bsp) -> HashSet<String> {
    let mut texinfos = HashSet::<u16>::new();

    for faces in &bsp.faces {
        texinfos.insert(faces.texinfo);
    }

    let mut texindices = HashSet::<u32>::new();

    for texinfo in &bsp.texinfo {
        texindices.insert(texinfo.texture_index);
    }

    let mut external_textures: HashSet<String> = HashSet::<String>::new();

    for texindex in texindices {
        // 0 offset means external wad
        let texture = &bsp.textures[texindex as usize];

        if texture.mip_offsets[0] == 0 {
            external_textures.insert(texture.texture_name.get_string_standard());
        }
    }

    external_textures
}

fn find_wad_file_from_wad_table(wad_table: &WadTable, tex: &str) -> eyre::Result<String> {
    for (key, value) in wad_table {
        if value.get(tex).is_some() {
            return Ok(key.to_string());
        }
    }

    return err!("cannot find texture `{}` from wad table", tex);
}

fn generate_wad_table(gamemod_dir: &Path) -> eyre::Result<WadTable> {
    let root_folder = gamemod_dir;

    let mut wad_table = WadTable::new();

    let huh = fs::read_dir(&root_folder)?;

    huh.filter_map(|read_dir| read_dir.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().is_some() && path.extension().unwrap() == "wad")
        .for_each(|path| {
            let wad_file_name = path.file_name().unwrap().to_str().unwrap().to_string();

            // Some wad files are retarded and they are not even WAD3
            // This means my wad lib should be very correct
            let wad = match Wad::from_file(&path) {
                Ok(wad) => wad,
                Err(_) => return,
            };

            wad_table.push((wad_file_name, HashSet::new()));
            let l = wad_table.len();

            wad.entries.iter().for_each(|wad_entry| {
                wad_table[l - 1].1.insert(wad_entry.texture_name());
            });
        });

    // vector so we can sort it
    wad_table.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(wad_table)
}

fn resmake_header() -> String {
    format!(
        "\
// .res generated by gchimp ResMake
// https://github.com/khanghugo/gchimp
// Generated date: {}
",
        Local::now().to_rfc2822()
    )
}

// should not be used directly because this does not have any checks
fn resmake_single_bsp(
    bsp: &Bsp,
    bsp_path: &Path,
    wad_table: Option<&WadTable>,
    options: &ResMakeOptions,
) -> eyre::Result<String> {
    let bsp_name = bsp_path.file_stem().unwrap().to_str().unwrap();

    let mut res_file = String::new();

    res_file += resmake_header().as_str();

    // models
    // "model": "models/.../.../models.mdl"
    let mut used_models = HashSet::<&str>::new();

    for entity in &bsp.entities {
        if let Some(classname) = entity.get("classname") {
            if MODEL_ENTITIES.contains(&classname.as_str()) {
                if let Some(model) = entity.get("model") {
                    if model.ends_with(".mdl") {
                        used_models.insert(model);
                    }
                }
            }
        }
    }

    if !used_models.is_empty() {
        res_file += "\n";
        res_file += "// models \n";

        let mut used_models = used_models.into_iter().collect::<Vec<_>>();
        used_models.sort();

        for used_model in used_models {
            res_file += used_model;
            res_file += "\n";
        }
    }

    // sound
    // "message": "audio.wav"
    // prefix for folder "sounds" is not included.
    let mut used_sounds = HashSet::<&str>::new();

    for entity in &bsp.entities {
        if let Some(classname) = entity.get("classname") {
            if SOUND_ENTITIES.contains(&classname.as_str()) {
                if let Some(message) = entity.get("message") {
                    if message.ends_with(".wav") {
                        used_sounds.insert(&message);
                    }
                }
            }
        }
    }

    if !used_sounds.is_empty() {
        res_file += "\n";
        res_file += "// sound\n";

        let mut used_sounds = used_sounds.into_iter().collect::<Vec<_>>();
        used_sounds.sort();

        for used_sound in used_sounds {
            res_file += used_sound;
            res_file += "\n";
        }
    }

    // gfx
    // skybox and detail textures
    res_file += "\n";
    res_file += "// gfx\n";

    // entity 0 is worldbrush and we can get the skybox from there
    let entity0 = &bsp.entities[0];

    let has_detail_textures = if let Some(classname) = entity0.get("classname") {
        if classname != "worldspawn" {
            return err!("first entity is not a worldbrush entity");
        }

        let skyname = entity0
            .get("skyname")
            .map(|skyname| skyname.to_string())
            .unwrap_or("desert".to_string());

        let base_skyname = format!("gfx/env/{}", skyname);

        // skybox
        ["bk", "dn", "ft", "lf", "rt", "up"]
            .iter()
            .for_each(|suffix| {
                res_file += format!("{}{}.tga\n", base_skyname, suffix).as_str();
            });

        // detail texture
        let detail_texture_file_path = bsp_path.with_file_name(format!("{}_detail.txt", bsp_name));

        if detail_texture_file_path.exists() {
            let mut used_detail_textures = HashSet::<String>::new();

            let mut detail_texture_file = match OpenOptions::new()
                .read(true)
                .open(&detail_texture_file_path)
            {
                Ok(a) => a,
                Err(err) => {
                    return err!(
                        "cannot open detail texture file `{}` for bsp file `{}`: {err}",
                        detail_texture_file_path.display(),
                        bsp_path.display()
                    )
                }
            };

            let mut s = String::new();
            detail_texture_file.read_to_string(&mut s)?;

            let base_detail_textures = "gfx";
            res_file += "\n";

            s.lines().for_each(|line| {
                if let Some(detail_texture) = line.split_ascii_whitespace().nth(1) {
                    if !detail_texture.is_empty() {
                        used_detail_textures
                            .insert(format!("{}/{}.tga", base_detail_textures, detail_texture));
                    }
                }
            });

            let mut used_detail_textures = used_detail_textures.into_iter().collect::<Vec<_>>();
            used_detail_textures.sort();

            for used_detail_texture in used_detail_textures {
                res_file += used_detail_texture.as_str();
                res_file += "\n";
            }

            true
        } else {
            false
        }
    } else {
        false
    };

    // sprites
    // "model": "sprites/.../.../sprite.spr"
    let mut used_sprites = HashSet::<&str>::new();

    for entity in &bsp.entities {
        if let Some(classname) = entity.get("classname") {
            if SPRITE_ENTITIES.contains(&classname.as_str()) {
                // env_sprite
                // env_glow
                if let Some(model) = entity.get("model") {
                    // some of sprite entities are used for displaying model so this check is to make sure
                    if model.ends_with(".spr") {
                        used_sprites.insert(&model);
                    }
                }
                // env_beam
                else if let Some(texture) = entity.get("texture") {
                    used_sprites.insert(&texture);
                }
            }
        }
    }

    if !used_sprites.is_empty() {
        res_file += "\n";
        res_file += "// sprites\n";

        let mut used_sprites = used_sprites.into_iter().collect::<Vec<_>>();
        used_sprites.sort();

        for used_sprite in used_sprites {
            res_file += used_sprite;
            res_file += "\n";
        }
    }

    // maps
    // .bsp, .res, detail texture file
    res_file += "\n";
    res_file += "// maps\n";

    // there are no checks here, don't use this function by itself
    let bsp_file_components = bsp_path.components().collect::<Vec<_>>();
    let l = bsp_file_components.len();

    // .bsp
    res_file += format!(
        "{}/{}/{}\n",
        bsp_file_components[l - 3].as_os_str().to_str().unwrap(),
        bsp_file_components[l - 2].as_os_str().to_str().unwrap(),
        bsp_file_components[l - 1].as_os_str().to_str().unwrap()
    )
    .as_str();

    // _detail.txt
    if has_detail_textures {
        res_file += format!(
            "{}/{}/{}_detail.txt\n",
            bsp_file_components[l - 3].as_os_str().to_str().unwrap(),
            bsp_file_components[l - 2].as_os_str().to_str().unwrap(),
            bsp_name
        )
        .as_str();
    }

    // .res
    res_file += format!(
        "{}/{}/{}.res\n",
        bsp_file_components[l - 3].as_os_str().to_str().unwrap(),
        bsp_file_components[l - 2].as_os_str().to_str().unwrap(),
        bsp_name
    )
    .as_str();

    // .wad
    let external_textures = need_external_wad(&bsp);

    if !external_textures.is_empty() && options.wad_check {
        if let Some(wad_table) = wad_table {
            let mut used_wads = HashSet::<String>::new();

            for used_texture in external_textures {
                let x = find_wad_file_from_wad_table(wad_table, used_texture.as_str())?;
                used_wads.insert(x);
            }

            res_file += "\n";
            res_file += "// wads\n";

            let mut used_wads = used_wads.into_iter().collect::<Vec<_>>();
            used_wads.sort();

            for used_wad in used_wads {
                res_file += used_wad.as_str();
                res_file += "\n";
            }
        } else {
            return err!(
                "bsp file `{}` needs external wad but none supplied",
                bsp_path.display()
            );
        }
    }

    Ok(res_file)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::modules::resmake::ResMake;

    #[test]
    fn no_path() {
        let resmake = ResMake::new();

        assert!(resmake.single_bsp().is_err());
    }

    #[test]
    fn run_external_wad_no_find() {
        let path = PathBuf::from("/home/khang/bxt/_game_native/valve/maps/c2a2c.bsp");
        let mut binding = ResMake::new();
        let resmake = binding.bsp_file(path);

        println!("{}", resmake._get_resmake_single_bsp_string().unwrap())
    }

    #[test]
    fn run_external_wad_yes_find() {
        let path = PathBuf::from("/home/khang/bxt/_game_native/valve/maps/c2a2c.bsp");
        let mut binding = ResMake::new();
        let resmake = binding.bsp_file(path).wad_check(true);

        println!("{}", resmake._get_resmake_single_bsp_string().unwrap())
    }
}
