use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use bsp::Bsp;
use chrono::Local;
use eyre::OptionExt;
use wad::types::Wad;
use zip::{write::SimpleFileOptions, ZipWriter};

use rayon::prelude::*;

use crate::{
    err,
    utils::{
        constants::{MODEL_ENTITIES, SOUND_ENTITIES, SPRITE_ENTITIES},
        misc::{search_game_resource, DefaultResource, COMMON_GAME_MODS},
    },
};

pub struct ResMakeOptions {
    /// Whether to generate RES
    pub res: bool,
    /// Whether to make a ZIP archive
    pub zip: bool,
    /// Whether to include external WAD inside RES and ZIP
    pub wad_check: bool,
    /// Whether to include default resource inside base game
    pub include_default_resource: bool,
    /// Wheter to ignore errors when resource is not found
    pub zip_ignore_missing: bool,
    /// Whether to creates one new linked WAD with textures from external WADs
    pub create_linked_wad: bool,
    /// Whether to skip making resources for BSP that already has RES
    pub skip_created_res: bool,
}

impl Default for ResMakeOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ResMakeOptions {
    pub fn new() -> Self {
        Self {
            res: true,
            zip: true,
            wad_check: false,
            include_default_resource: false,
            zip_ignore_missing: true,
            create_linked_wad: false,
            skip_created_res: false,
        }
    }
}

// need to be a vector so we can sort it by wad file name
// (wad file name, set of textures)
// wad file name is the path to the wad
// because the wad file can be from a different game mod
/// (Absolute path to WAD, Set of textures inside WAD)
type WadTable = Vec<(PathBuf, HashSet<String>)>;

pub struct ResMake {
    bsp_file: Option<PathBuf>,
    // For mass processing
    bsp_folder: Option<PathBuf>,
    root_folder: Option<PathBuf>,
    options: ResMakeOptions,
}

impl Default for ResMake {
    fn default() -> Self {
        Self::new()
    }
}

impl ResMake {
    pub fn new() -> Self {
        Self {
            bsp_file: None,
            bsp_folder: None,
            root_folder: None,
            options: ResMakeOptions::default(),
        }
    }

    pub fn bsp_file(&mut self, path: impl AsRef<Path> + Into<PathBuf>) -> &mut Self {
        self.bsp_file = Some(path.into());

        self
    }

    pub fn bsp_folder(&mut self, path: impl AsRef<Path> + Into<PathBuf>) -> &mut Self {
        self.bsp_folder = Some(path.into());

        self
    }

    pub fn root_folder(&mut self, path: impl AsRef<Path> + Into<PathBuf>) -> &mut Self {
        self.root_folder = Some(path.into());

        self
    }

    pub fn skip_created_res(&mut self, v: bool) -> &mut Self {
        self.options.skip_created_res = v;

        self
    }

    pub fn res(&mut self, v: bool) -> &mut Self {
        self.options.res = v;

        self
    }

    pub fn zip(&mut self, v: bool) -> &mut Self {
        self.options.zip = v;

        self
    }

    pub fn wad_check(&mut self, v: bool) -> &mut Self {
        self.options.wad_check = v;

        self
    }

    pub fn include_default_resource(&mut self, v: bool) -> &mut Self {
        self.options.include_default_resource = v;

        self
    }

    pub fn zip_ignore_missing(&mut self, v: bool) -> &mut Self {
        self.options.zip_ignore_missing = v;

        self
    }

    pub fn create_linked_wad(&mut self, v: bool) -> &mut Self {
        self.options.create_linked_wad = v;

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

    /// Returns the game directory, aka /path/to/hl.exe
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

            if maps_folder.parent().unwrap().parent().is_none() {
                return err!("game mod folder is not inside a game directory containing hl.exe");
            }
        }

        Ok(path
            .parent()
            .and_then(|bsp_folder| bsp_folder.parent())
            .and_then(|gamemod_folder| gamemod_folder.parent())
            .unwrap()
            .to_path_buf())
    }

    fn generate_wad_table(&self) -> eyre::Result<WadTable> {
        let game_dir = if self.bsp_file.is_some() {
            self.check_bsp_file()?;
            self.check_bsp_file_parent()?
        } else if let Some(root_folder) = &self.root_folder {
            root_folder.to_path_buf()
        } else {
            return err!("no folder set");
        };

        generate_wad_table(&game_dir)
    }

    // pub fn _get_resmake_single_bsp_string(&self) -> eyre::Result<String> {
    //     self.check_bsp_file()?;
    //     self.check_bsp_file_parent()?;

    //     let wad_table = if self.options.wad_check {
    //         Some(self.generate_wad_table()?)
    //     } else {
    //         None
    //     };

    //     let bsp = Bsp::from_file(self.bsp_file.as_ref().unwrap())?;

    //     resmake_single_bsp(
    //         &bsp,
    //         self.bsp_file.as_ref().unwrap(),
    //         wad_table.as_ref(),
    //         &self.options,
    //     )
    // }

    pub fn run(&self) -> eyre::Result<()> {
        self.check_bsp_file()?;

        let bsp_path = self.bsp_file.as_ref().unwrap();

        let res_exists = bsp_path.with_extension("res").exists();
        let skip_created_res = if res_exists {
            self.options.skip_created_res
        } else {
            false
        };

        if skip_created_res {
            return Ok(());
        }

        if self.options.wad_check || self.options.zip {
            self.check_bsp_file_parent()?;
        }

        let wad_table = if self.options.wad_check {
            Some(self.generate_wad_table()?)
        } else {
            None
        };

        let bsp = Bsp::from_file(bsp_path)?;

        if self.options.res {
            let res_string = resmake_single_bsp(&bsp, bsp_path, wad_table.as_ref(), &self.options)?;

            let out_path = bsp_path.with_extension("res");
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(out_path)?;

            file.write_all(res_string.as_bytes())?;
            file.flush()?;
        }

        if self.options.zip {
            let res_bytes = resmake_zip_res(&bsp, bsp_path, wad_table.as_ref(), &self.options)?;

            let out_path = bsp_path.with_extension("zip");
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(out_path)?;

            file.write_all(&res_bytes)?;
            file.flush()?;
        }

        Ok(())
    }

    pub fn run_folder(&self) -> eyre::Result<()> {
        let Some(bsp_folder) = &self.bsp_folder else {
            return err!("no bsp folder given for mass processing");
        };

        if !bsp_folder.is_dir() {
            return err!("given path `{}` is not a folder", bsp_folder.display());
        }

        let bsp_folder_name = bsp_folder
            .file_name()
            .ok_or_eyre("given path does not have a name")?;

        if bsp_folder_name != "maps" {
            return err!("given path is not a `maps` folder");
        }

        let gamemod_dir = bsp_folder
            .parent()
            .ok_or_eyre("`maps` folder does not have a parent, should this happen?")?;

        let game_dir = gamemod_dir
            .parent()
            .ok_or_eyre("game mod folder does not have a parent")?;

        let wad_table = if self.options.wad_check {
            generate_wad_table(game_dir).ok()
        } else {
            None
        };

        let bsp_paths: Vec<PathBuf> = std::fs::read_dir(bsp_folder)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let entry_path = entry.path();

                if !entry_path.is_file() {
                    return None;
                }

                let extension = entry_path.extension()?;

                if extension == "bsp" {
                    return Some(entry_path);
                }

                None
            })
            .collect();

        let counter = Arc::new(Mutex::new(0u32));

        let start_processing = |skip: bool, bsp_path: &Path| {
            let _ = counter.lock().map(|mut v| {
                *v = *v + 1;
                println!(
                    "{} processing {}/{} : {}",
                    if skip { "Skip" } else { "Start" },
                    *v,
                    bsp_paths.len(),
                    bsp_path.display()
                );
            });
        };

        let multithread = std::env::var("GCHIMP_RESMAKE_MULTITHREAD").is_ok();

        let good_fucking_god_rust_you_are_so_good_at_inference = |bsp_path: &PathBuf| {
            let res_exists = bsp_path.with_extension("res").exists();
            let zip_exists = bsp_path.with_extension("zip").exists();

            let skip_created_res = if res_exists {
                if self.options.zip {
                    zip_exists && self.options.skip_created_res
                } else {
                    self.options.skip_created_res
                }
            } else {
                false
            };

            if skip_created_res {
                start_processing(true, &bsp_path);
                return Ok(());
            }

            start_processing(false, &bsp_path);

            let bsp = Bsp::from_file(bsp_path)?;

            if self.options.res {
                let res_string =
                    resmake_single_bsp(&bsp, bsp_path, wad_table.as_ref(), &self.options)?;

                let out_path = bsp_path.with_extension("res");
                let mut file = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(out_path)?;

                file.write_all(res_string.as_bytes())?;
                file.flush()?;
            }

            if self.options.zip {
                let res_bytes = resmake_zip_res(&bsp, bsp_path, wad_table.as_ref(), &self.options)?;

                let out_path = bsp_path.with_extension("zip");
                let mut file = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(out_path)?;

                file.write_all(&res_bytes)?;
                file.flush()?;
            }

            Ok(())
        };

        if multithread {
            bsp_paths
                .par_iter()
                .map(|bsp_path| good_fucking_god_rust_you_are_so_good_at_inference(bsp_path))
                .collect::<eyre::Result<Vec<_>>>()?;
        } else {
            bsp_paths
                .iter()
                .map(|bsp_path| good_fucking_god_rust_you_are_so_good_at_inference(bsp_path))
                .collect::<eyre::Result<Vec<_>>>()?;
        };

        Ok(())
    }
}

fn need_external_wad(bsp: &Bsp) -> HashSet<String> {
    let mut external_textures: HashSet<String> = HashSet::<String>::new();

    for texture in &bsp.textures {
        // 0 offset means external wad
        let texture_name = texture.texture_name.get_string_standard();

        if texture.is_external() {
            external_textures.insert(texture_name);
        }
    }

    external_textures
}

/// Returns the wad file inside the wad table
fn find_wad_file_from_wad_table<'a>(wad_table: &'a WadTable, tex: &str) -> Option<&'a Path> {
    for (key, value) in wad_table {
        if value.get(tex).is_some() {
            return Some(key);
        }
    }

    println!("cannot find texture `{}` from wad table", tex);

    None
}

fn generate_wad_table(game_dir: &Path) -> eyre::Result<WadTable> {
    let root_folder = game_dir;

    let mut wad_table = WadTable::new();

    COMMON_GAME_MODS.iter().for_each(|gamemod| {
        let Ok(huh) = fs::read_dir(root_folder.join(gamemod)) else {
            return;
        };

        huh.filter_map(|read_dir| read_dir.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| path.extension().is_some() && path.extension().unwrap() == "wad")
            .for_each(|path| {
                // Some wad files are retarded and they are not even WAD3
                // This means my wad lib should be very correct
                let wad = match Wad::from_file(&path) {
                    Ok(wad) => wad,
                    Err(_) => return,
                };

                wad_table.push((path.to_path_buf(), HashSet::new()));
                let l = wad_table.len();

                wad.entries.iter().for_each(|wad_entry| {
                    wad_table[l - 1].1.insert(wad_entry.texture_name_standard());
                });
            });
    });

    // vector so we can sort it
    wad_table.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(wad_table)
}

fn resmake_res_header(entry_count: i32) -> String {
    format!(
        "\
// .res generated by gchimp ResMake
// https://github.com/khanghugo/gchimp
// Generated date: {}
// Entry count: {}
",
        Local::now().to_rfc2822(),
        entry_count
    )
}

fn resmake_zip_comment() -> String {
    format!(
        "\
Archive generated by gchimp ResMake
https://github.com/khanghugo/gchimp
Generated date: {}
",
        Local::now().to_rfc2822()
    )
}
#[inline]
fn filter_default<T>(i: Vec<T>) -> Vec<T>
where
    T: Into<String> + AsRef<str> + Ord,
{
    i.into_iter()
        .filter(|s| !DefaultResource.is_default_resource(s))
        .collect::<Vec<_>>()
}

#[inline]
fn sort<T>(i: Vec<T>) -> Vec<T>
where
    T: Into<String> + AsRef<str> + Ord,
{
    let mut i = i;
    i.sort();
    i
}

fn help_a_friend_out(s: &str) -> &str {
    for what in COMMON_GAME_MODS {
        if let Some(huh) = s.strip_prefix(what) {
            // stripping backslash
            return &huh[1..];
        }
    }

    return s;
}

fn get_models(bsp: &Bsp) -> HashSet<String> {
    let mut used_models = HashSet::<String>::new();

    for entity in &bsp.entities {
        if let Some(classname) = entity.get("classname") {
            if MODEL_ENTITIES.contains(&classname.as_str()) {
                if let Some(model) = entity.get("model") {
                    if model.ends_with(".mdl") {
                        let model = help_a_friend_out(model);

                        used_models.insert(model.to_string());
                    }
                }
            }
        }
    }

    used_models
}

fn get_sound(bsp: &Bsp) -> HashSet<String> {
    let mut used_sounds = HashSet::<String>::new();

    for entity in &bsp.entities {
        if let Some(classname) = entity.get("classname") {
            if SOUND_ENTITIES.contains(&classname.as_str()) {
                if let Some(message) = entity.get("message") {
                    if message.ends_with(".wav") {
                        // need to pad "sound" at the beginning
                        let sound_path = format!("sound/{}", message);

                        used_sounds.insert(sound_path);
                    }
                }
            }
        }
    }

    used_sounds
}

struct GetGfxResult {
    gfx: HashSet<String>,
    has_detailed_textures: bool,
}

fn get_gfx(bsp: &Bsp, bsp_path: &Path, bsp_name: &str) -> eyre::Result<GetGfxResult> {
    let mut used_gfx = HashSet::<String>::new();
    let mut has_detailed_textures = false;

    // entity 0 is worldbrush and we can get the skybox from there
    let entity0 = &bsp.entities[0];

    if let Some(classname) = entity0.get("classname") {
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
                let sky_part = format!("{}{}.tga", base_skyname, suffix);

                used_gfx.insert(sky_part);
            });

        // detail texture
        let detail_texture_file_path = bsp_path.with_file_name(format!("{}_detail.txt", bsp_name));

        if detail_texture_file_path.exists() {
            has_detailed_textures = true;

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

            s.lines().for_each(|line| {
                // ignore commments
                if line.starts_with("//") {
                    return;
                }

                if let Some(detail_texture) = line.split_ascii_whitespace().nth(1) {
                    let detail_file = format!("{}/{}.tga", base_detail_textures, detail_texture);

                    if !detail_texture.is_empty() {
                        used_gfx.insert(detail_file);
                    }
                }
            });
        }
    }

    Ok(GetGfxResult {
        gfx: used_gfx,
        has_detailed_textures,
    })
}

fn get_sprites(bsp: &Bsp) -> HashSet<String> {
    let mut used_sprites = HashSet::<String>::new();

    for entity in &bsp.entities {
        if let Some(classname) = entity.get("classname") {
            if SPRITE_ENTITIES.contains(&classname.as_str()) {
                // env_sprite
                // env_glow
                if let Some(model) = entity.get("model") {
                    // some of sprite entities are used for displaying model so this check is to make sure
                    if model.ends_with(".spr") {
                        let model = help_a_friend_out(model);

                        used_sprites.insert(model.to_string());
                    }
                }
                // env_beam
                else if let Some(texture) = entity.get("texture") {
                    let texture = help_a_friend_out(texture);

                    used_sprites.insert(texture.to_string());
                }
            }
        }
    }

    used_sprites
}

fn get_wads(
    external_textures: &HashSet<String>,
    wad_table: &WadTable,
) -> eyre::Result<HashSet<String>> {
    let mut used_wads = HashSet::<String>::new();

    for used_texture in external_textures {
        let Some(x) = find_wad_file_from_wad_table(wad_table, used_texture.as_str()) else {
            continue;
        };
        let wad_file = x.file_name().unwrap().to_str().unwrap().to_string();
        used_wads.insert(wad_file);
    }

    Ok(used_wads)
}

/// Returns the WAD path starting from gamemod dir
fn create_linked_wad(
    bsp_path: &Path,
    external_textures: &[String],
    wad_table: &WadTable,
) -> eyre::Result<String> {
    let mut wad = Wad::new();
    let mut wad_file_table: HashMap<&Path, Wad> = HashMap::new();

    let game_mod = bsp_path.parent().unwrap().parent().unwrap();

    for used_texture in external_textures {
        let Some(x) = find_wad_file_from_wad_table(wad_table, used_texture.as_str()) else {
            continue;
        };

        let wad_entry = wad_file_table
            .entry(x)
            .or_insert(Wad::from_file(game_mod.join(x))?);

        wad_entry
            .entries
            .iter()
            .find(|texture_entry| {
                texture_entry.texture_name().as_str() == used_texture
                    || texture_entry.texture_name_standard().as_str() == used_texture
            })
            .map(|entry| {
                wad.entries.push(entry.clone());
                wad.header.num_dirs += 1;
            });
    }

    let bsp_name = bsp_path.file_stem().unwrap().to_str().unwrap();
    let wad_name = [bsp_name, ".wad"].concat();
    let out_wad_path = game_mod.join(&wad_name);

    wad.write_to_file(out_wad_path)?;

    Ok(wad_name)
}

/// Should not be used directly because this does not have any checks
pub fn resmake_single_bsp(
    bsp: &Bsp,
    bsp_path: &Path,
    wad_table: Option<&WadTable>,
    options: &ResMakeOptions,
) -> eyre::Result<String> {
    let resources = find_resource(bsp, bsp_path, wad_table, options.wad_check)?;
    let resources = if options.include_default_resource {
        resources
    } else {
        resources.filter_default_resource()
    };

    let FindResource {
        bsp_name: _bsp_name,
        bsp_path: _bsp_path,
        models,
        sound,
        gfx,
        has_detailed_textures: _x,
        sprites,
        wads,
        external_textures,
    } = resources.sort_resource();

    let mut res_file = String::new();

    let mut entry_count = 0;

    {
        // models
        // "model": "models/.../.../models.mdl"
        let used_models = models;

        if !used_models.is_empty() {
            res_file += "\n";
            res_file += "// models \n";

            for used_model in used_models {
                res_file += used_model.as_str();
                res_file += "\n";

                entry_count += 1;
            }
        }
    }

    {
        // sound
        // "message": "audio.wav"
        // prefix for folder "sounds" is not included.
        // so we need to include it
        let used_sounds = sound;

        if !used_sounds.is_empty() {
            res_file += "\n";
            res_file += "// sound\n";

            for used_sound in used_sounds {
                res_file += used_sound.as_str();
                res_file += "\n";

                entry_count += 1;
            }
        }
    }

    {
        // gfx
        // skybox and detail textures
        let used_gfx = gfx;

        if !used_gfx.is_empty() {
            res_file += "\n";
            res_file += "// gfx\n";

            for used_gfx_singular in used_gfx {
                res_file += used_gfx_singular.as_str();
                res_file += "\n";

                entry_count += 1;
            }
        }
    }

    {
        // sprites
        // "model": "sprites/.../.../sprite.spr"
        let used_sprites = sprites;

        if !used_sprites.is_empty() {
            res_file += "\n";
            res_file += "// sprites\n";

            for used_sprite in used_sprites {
                res_file += used_sprite.as_str();
                res_file += "\n";

                entry_count += 1;
            }
        }
    }
    // no need to add .bsp and .res because they are no needed
    // {
    //     // maps
    //     // .bsp, .res, detail texture file
    //     res_file += "\n";
    //     res_file += "// maps\n";

    //     // .bsp
    //     res_file += format!("maps/{}.bsp\n", bsp_name).as_str();

    //     entry_count += 1;

    //     // _detail.txt
    //     if has_detail_textures {
    //         res_file += format!("maps/{}_detail.txt\n", bsp_name).as_str();

    //         entry_count += 1;
    //     }

    //     // .res
    //     res_file += format!("maps/{}.res\n", bsp_name).as_str();

    //     entry_count += 1;
    // }

    // .wad
    {
        if !wads.is_empty() {
            res_file += "\n";
            res_file += "// wads\n";

            // if zip and then create linked wad then just use the linked wad instead of external wads
            if options.zip && options.create_linked_wad {
                // wad_table surely has some values here because of the find_resource function
                let wad_path = create_linked_wad(bsp_path, &external_textures, wad_table.unwrap())?;

                res_file += wad_path.as_str();
                res_file += "\n";

                entry_count += 1;
            } else {
                // the wad table is storing absolute path, so here we will convert to relative path
                // wad is usually inside game mod, so we can just take the file name directly
                let used_wads_paths: Vec<&Path> = wads.iter().map(|s| Path::new(s)).collect();
                let used_wads_relative_paths: Vec<&str> = used_wads_paths
                    .iter()
                    .map(|path| path.file_name().unwrap().to_str().unwrap())
                    .collect();

                for used_wad in used_wads_relative_paths {
                    res_file += used_wad;
                    res_file += "\n";

                    entry_count += 1;
                }
            }
        }
    }

    // add header when everything is done
    res_file.insert_str(0, resmake_res_header(entry_count).as_str());

    if entry_count == 0 {
        res_file += "\n// res file is empty\n"
    }

    Ok(res_file)
}

type ResourceList = Vec<String>;

struct FindResource {
    /// "cstrike/maps/my_map.bsp" becomes "my_map"
    bsp_name: String,
    bsp_path: PathBuf,
    models: ResourceList,
    sound: ResourceList,
    gfx: ResourceList,
    has_detailed_textures: bool,
    sprites: ResourceList,
    // wad contains absolute path
    wads: ResourceList,
    external_textures: ResourceList,
}

impl FindResource {
    fn filter_default_resource(self) -> Self {
        let Self {
            bsp_name,
            bsp_path,
            models,
            sound,
            gfx,
            has_detailed_textures,
            sprites,
            wads,
            external_textures,
        } = self;

        let models = filter_default(models);
        let sound = filter_default(sound);
        let gfx = filter_default(gfx);
        let sprites = filter_default(sprites);
        let wads = filter_default(wads);

        Self {
            bsp_name,
            bsp_path,
            models,
            sound,
            gfx,
            has_detailed_textures,
            sprites,
            wads,
            external_textures,
        }
    }

    fn sort_resource(self) -> Self {
        let Self {
            bsp_name,
            bsp_path,
            models,
            sound,
            gfx,
            has_detailed_textures,
            sprites,
            wads,
            external_textures,
        } = self;

        let models = sort(models);
        let sound = sort(sound);
        let gfx = sort(gfx);
        let sprites = sort(sprites);
        let wads = sort(wads);

        Self {
            bsp_name,
            bsp_path,
            models,
            sound,
            gfx,
            has_detailed_textures,
            sprites,
            wads,
            external_textures,
        }
    }
}

fn find_resource(
    bsp: &Bsp,
    bsp_path: &Path,
    wad_table: Option<&WadTable>,
    wad_check: bool,
) -> eyre::Result<FindResource> {
    let bsp_name = bsp_path.file_stem().unwrap().to_str().unwrap();

    let to_vec = move |i: HashSet<String>| i.into_iter().collect::<Vec<_>>();

    let models = get_models(bsp);
    let sound = get_sound(bsp);
    let GetGfxResult {
        gfx,
        has_detailed_textures,
    } = get_gfx(bsp, bsp_path, bsp_name)?;
    let sprites = get_sprites(bsp);

    let (wads, external_textures) = {
        let external_textures = need_external_wad(bsp);

        if wad_check && !external_textures.is_empty() && wad_table.is_some() {
            (
                to_vec(get_wads(&external_textures, wad_table.unwrap())?),
                external_textures,
            )
        } else {
            (vec![], external_textures)
        }
    };

    let external_textures = to_vec(external_textures);

    Ok(FindResource {
        bsp_name: bsp_name.to_string(),
        bsp_path: bsp_path.to_path_buf(),
        models: to_vec(models),
        sound: to_vec(sound),
        gfx: to_vec(gfx),
        has_detailed_textures,
        sprites: to_vec(sprites),
        wads,
        external_textures,
    })
}

fn resmake_zip_res(
    bsp: &Bsp,
    bsp_path: &Path,
    wad_table: Option<&WadTable>,
    options: &ResMakeOptions,
) -> eyre::Result<Vec<u8>> {
    let resources = find_resource(bsp, bsp_path, wad_table, options.wad_check)?;
    let resources = if options.include_default_resource {
        resources
    } else {
        resources.filter_default_resource()
    };

    let FindResource {
        bsp_name,
        bsp_path,
        models,
        sound,
        gfx,
        has_detailed_textures: _x,
        sprites,
        wads,
        external_textures: _,
    } = resources.sort_resource();

    // path/to/hl/cstrike/maps/map.bsp -> path/to/hl/cstrike
    // can also work for any arbitrary folder
    let gamemod_path = bsp_path.parent().unwrap().parent().unwrap();
    let gamemod_name = gamemod_path.file_name().unwrap().to_str().unwrap();
    let gamedir_path = gamemod_path.parent().unwrap();

    // group all files in one
    let all_files = [models, sound, gfx, sprites].concat();
    let mut all_files = if !wads.is_empty() {
        // linked wad is already created in the .res step
        if options.create_linked_wad {
            let wad_path = [&bsp_name, ".wad"].concat();

            [all_files, vec![wad_path]].concat()
        } else {
            [all_files, wads].concat()
        }
    } else {
        all_files
    };

    // now get the bsp, maybe res, and maybe _detail.txt
    {
        // these files are guaranteed to be inside the same folder as the .bsp
        // so, we dont need to search them with the complicated way
        let relative_bsp_path = format!("maps/{}.bsp", bsp_name);
        let relative_res_path = format!("maps/{}.res", bsp_name);
        let relative_detail_path = format!("maps/{}_detail.txt", bsp_name);

        all_files.push(relative_bsp_path);

        let absolute_res_path = gamemod_path.join(&relative_res_path);
        let absolute_detail_path = gamemod_path.join(&relative_detail_path);

        if absolute_res_path.exists() {
            all_files.push(relative_res_path);
        }

        if absolute_detail_path.exists() {
            all_files.push(relative_detail_path);
        }
    }

    let mut buf: Vec<u8> = vec![];
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));

    let zip_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut comment = resmake_zip_comment();

    // include typical resource files
    for relative_path in all_files {
        // let absolute_path = root_path.join(relative_path.as_str());
        // let absolute_path =
        let Some(absolute_path) = search_game_resource(
            gamedir_path,
            gamemod_name,
            Path::new(&relative_path),
            // good fucking god im smart enough to deal with this retardation
            false,
        ) else {
            let message = format!("Cannot find {}", relative_path);

            if options.zip_ignore_missing {
                println!("{}", message);
                continue;
            }

            return err!(message);
        };

        if !absolute_path.exists() {
            if options.zip_ignore_missing {
                comment += "\n";
                comment += format!("{} is missing\n", absolute_path.to_str().unwrap()).as_str();

                continue;
            }

            return err!("file {} does not exist", absolute_path.display());
        }

        let mut resource_file = OpenOptions::new().read(true).open(absolute_path)?;
        let mut resource_file_buffer: Vec<u8> = vec![];

        resource_file.read_to_end(&mut resource_file_buffer)?;

        zip.start_file(relative_path, zip_options)?;

        // need to use write_all, for some reasons
        zip.write_all(&resource_file_buffer)?;
    }

    zip.set_comment(comment);

    zip.finish()?;

    Ok(buf)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use bsp::Bsp;

    use crate::modules::resmake::ResMake;

    use super::{resmake_zip_res, ResMakeOptions};

    #[test]
    fn no_path() {
        let resmake = ResMake::new();

        assert!(resmake.run().is_err());
    }

    // #[test]
    // fn run_external_wad_no_find() {
    //     let path = PathBuf::from("/home/khang/bxt/_game_native/valve/maps/c2a2c.bsp");
    //     let mut binding = ResMake::new();
    //     let resmake = binding.bsp_file(path);

    //     println!("{}", resmake._get_resmake_single_bsp_string().unwrap())
    // }

    // #[test]
    // fn run_external_wad_yes_find() {
    //     let path = PathBuf::from("/home/khang/bxt/_game_native/valve/maps/c2a2c.bsp");
    //     let mut binding = ResMake::new();
    //     let resmake = binding.bsp_file(path).wad_check(true);

    //     println!("{}", resmake._get_resmake_single_bsp_string().unwrap())
    // }

    #[test]
    fn run_zip() {
        let bsp_path = PathBuf::from("/home/khang/bxt/game_isolated/valve/maps/c0a0.bsp");
        let bsp = Bsp::from_file(&bsp_path).unwrap();

        resmake_zip_res(
            &bsp,
            &bsp_path,
            None,
            &ResMakeOptions {
                res: true,
                wad_check: false,
                include_default_resource: true,
                zip: true,
                zip_ignore_missing: true,
                create_linked_wad: true,
                skip_created_res: true,
            },
        )
        .unwrap();
    }

    #[test]
    fn run_mass1() {
        let path = PathBuf::from("/WD1/half-life/valve/maps/");
        let mut binding = ResMake::new();
        let resmake = binding
            .bsp_folder(path)
            .zip(true)
            .wad_check(true)
            .res(true)
            .include_default_resource(true)
            .create_linked_wad(true);

        resmake.run_folder().unwrap();
    }
}
