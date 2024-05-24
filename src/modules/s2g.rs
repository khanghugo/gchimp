use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use eyre::eyre;
use qc::Qc;
use smd::Smd;

/// 1 detect mdl
/// decompile mdl
/// look at the mdl's linked files
/// check whether linked files exist
/// split triangles from smd file
/// create qc file
/// call studiomdl.exe
///
/// extra steps:
/// bmp conversion

pub struct S2GSettings {
    studiomdl: Option<PathBuf>,
    crowbar: Option<PathBuf>,
    no_vtf: Option<PathBuf>,
    wine_prefix: Option<String>,
}

impl S2GSettings {
    pub fn new() -> Self {
        Self {
            studiomdl: None,
            crowbar: None,
            no_vtf: None,
            wine_prefix: None,
        }
    }

    pub fn studiomdl(&mut self, path: &str) -> &mut Self {
        self.studiomdl = Some(path.into());
        self
    }

    pub fn crowbar(&mut self, path: &str) -> &mut Self {
        self.crowbar = Some(path.into());
        self
    }

    pub fn no_vtf(&mut self, path: &str) -> &mut Self {
        self.no_vtf = Some(path.into());
        self
    }

    pub fn wine_prefix(&mut self, path: &str) -> &mut Self {
        self.wine_prefix = Some(path.into());
        self
    }

    fn check_studiomdl(&self) -> eyre::Result<()> {
        if self.studiomdl.is_none() {
            return Err(eyre!("No studiomdl.exe supplied"));
        }

        let studiomdl = self.studiomdl.as_ref().unwrap();
        let extension = studiomdl.extension();

        if cfg!(target_os = "windows") {
            if studiomdl.is_dir() || extension.is_none() {
                return Err(eyre!("Invalid studiomdl.exe"));
            }
        };

        Ok(())
    }

    fn check_crowbar(&self) -> eyre::Result<()> {
        if self.crowbar.is_none() {
            return Err(eyre!("No crowbar supplied"));
        }

        let crowbar = self.crowbar.as_ref().unwrap();
        let extension = crowbar.extension();

        if cfg!(target_os = "windows") {
            if crowbar.is_dir() || extension.is_none() {
                return Err(eyre!("Invalid crowbar"));
            }
        };

        Ok(())
    }

    fn check_no_vtf(&self) -> eyre::Result<()> {
        if self.no_vtf.is_none() {
            return Err(eyre!("No no_vtf supplied"));
        }

        let no_vtf = self.no_vtf.as_ref().unwrap();
        let extension = no_vtf.extension();

        if cfg!(target_os = "windows") {
            if no_vtf.is_dir() || extension.is_none() {
                return Err(eyre!("Invalid no_vtf"));
            }
        };

        Ok(())
    }

    fn check_wine_prefix(&self) -> eyre::Result<()> {
        if self.wine_prefix.is_none() {
            return Err(eyre!("No WINEPREFIX supplied"));
        }

        Ok(())
    }
}

pub struct S2GOptions {
    settings: S2GSettings,
    path: PathBuf,
    folder: bool,
    decompile: bool,
    /// For `folder` option, this will scan the .qc file instead of .mdl file
    use_qc: bool,
}

impl S2GOptions {
    pub fn new(path: &str, settings: S2GSettings) -> Self {
        Self {
            settings,
            path: PathBuf::from(path),
            folder: false,
            decompile: false,
            use_qc: false,
        }
    }

    /// Convert the whole folder. Not recursive.
    pub fn folder(&mut self, folder: bool) -> &mut Self {
        self.folder = folder;
        self
    }

    /// Decompile the model.
    pub fn decompile(&mut self, decompile: bool) -> &mut Self {
        self.decompile = decompile;
        self
    }

    /// Only load .qc files instead of .mdl files when scan for folders.
    ///
    /// Will enable `folder` option.
    pub fn use_qc(&mut self, use_qc: bool) -> &mut Self {
        self.use_qc = use_qc;

        if self.use_qc {
            self.folder(true);
        }

        self
    }

    pub fn work(&self) {}
}

struct Texture(PathBuf);

/// All of information related to a model decompiling process
struct Bucket {
    file_name: PathBuf,
    textures: Vec<Texture>,
    orig_qc: Qc,
    orig_smd: Vec<Smd>,
    converted_qc: Qc,
    converted_smd: Vec<Smd>,
}

fn find_extension_file_in_folder(path: &Path, ext: &str) -> std::io::Result<Vec<PathBuf>> {
    let rd = fs::read_dir(path)?;
    let paths = rd.filter_map(|path| path.ok()).map(|path| path.path());
    let ext_paths = paths
        .filter(|path| path.extension().is_some() && path.extension().unwrap() == ext)
        .collect();

    Ok(ext_paths)
}

fn decompile_mdl(mdl: &Path, settings: S2GSettings) -> eyre::Result<Output> {
    // Assume that all settings are valid.
    let crowbar = settings.crowbar.unwrap();

    // `./crowbar -p model.mdl`
    let command = vec![crowbar.to_str().unwrap(), "-p", mdl.to_str().unwrap()];

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(command)
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .args(command)
            .env("WINEPREFIX", settings.wine_prefix.unwrap())
            .output()
            .expect("failed to execute process")
    };

    Ok(output)
}
