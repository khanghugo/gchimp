use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    str::from_utf8,
    sync::{Arc, Mutex},
};

use constants::{
    CROWBAR_BINARY, GOLDSRC_SUFFIX, NO_VTF_BINARY, STUDIOMDL_BINARY, VTX_EXTENSION, VVD_EXTENSION,
};
use eyre::eyre;
use options::S2GOptions;
use qc::{BodyGroup, Qc, QcCommand};
use smd::Smd;

use rayon::prelude::*;

use crate::utils::{
    constants::STUDIOMDL_ERROR_PATTERN,
    img_stuffs::png_to_bmp_folder,
    misc::{
        find_files_with_ext_in_folder, fix_backslash, maybe_add_extension_to_string,
        relative_to_less_relative,
    },
    qc_stuffs::create_goldsrc_base_qc_from_source,
    run_bin::{run_crowbar, run_no_vtf, run_studiomdl},
    smd_stuffs::source_smd_to_goldsrc_smd,
};

mod constants;
pub mod options;

pub struct S2GSettings {
    studiomdl: PathBuf,
    crowbar: PathBuf,
    no_vtf: PathBuf,
    wineprefix: Option<String>,
}

impl Default for S2GSettings {
    fn default() -> Self {
        let current_exe_path = std::env::current_exe().unwrap();
        let path_to_dist = current_exe_path.parent().unwrap().join("dist");

        Self::new(&path_to_dist)
    }
}

// TODO: impl Default with included binaries.
impl S2GSettings {
    pub fn new(path_to_bin: &Path) -> Self {
        if !path_to_bin.exists() {
            panic!(
                "{} containing binaries for S2G does not exist.",
                path_to_bin.display()
            );
        }

        let studiomdl = path_to_bin.join(STUDIOMDL_BINARY);
        let crowbar = path_to_bin.join(CROWBAR_BINARY);
        let no_vtf = path_to_bin.join("no_vtf");
        let no_vtf = no_vtf.join(NO_VTF_BINARY);

        if !studiomdl.exists() {
            panic!(
                "Cannot find {} in {}",
                STUDIOMDL_BINARY,
                path_to_bin.display()
            );
        }

        if !crowbar.exists() {
            panic!(
                "Cannot find {} in {}",
                CROWBAR_BINARY,
                path_to_bin.display()
            );
        }

        if !no_vtf.exists() {
            panic!("Cannot find {} in {}", NO_VTF_BINARY, path_to_bin.display());
        }

        Self {
            studiomdl,
            crowbar,
            no_vtf,
            wineprefix: None,
        }
    }

    #[allow(dead_code)]
    pub fn studiomdl(&mut self, path: &str) -> &mut Self {
        self.studiomdl = path.into();
        self
    }

    #[allow(dead_code)]
    pub fn crowbar(&mut self, path: &str) -> &mut Self {
        self.crowbar = path.into();
        self
    }

    #[allow(dead_code)]
    pub fn no_vtf(&mut self, path: &str) -> &mut Self {
        self.no_vtf = path.into();
        self
    }

    #[allow(dead_code)]
    pub fn wineprefix(&mut self, path: Option<String>) -> &mut Self {
        self.wineprefix = path;
        self
    }

    #[cfg(target_os = "linux")]
    fn check_wineprefix(&self) -> eyre::Result<()> {
        if self.wineprefix.is_none() || self.wineprefix.as_ref().unwrap().is_empty() {
            return Err(eyre!("No WINEPREFIX supplied"));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct S2GSync {
    stdout: Arc<Mutex<String>>,
    is_done: Arc<Mutex<bool>>,
}

impl S2GSync {
    fn new() -> Self {
        Self {
            stdout: Arc::new(Mutex::new(String::new())),
            // is_done = true initially
            is_done: Arc::new(Mutex::new(true)),
        }
    }

    pub fn stdout(&self) -> &Arc<Mutex<String>> {
        &self.stdout
    }

    pub fn is_done(&self) -> &Arc<Mutex<bool>> {
        &self.is_done
    }
}

impl Default for S2GSync {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct S2GSteps {
    pub decompile: bool,
    pub vtf: bool,
    pub bmp: bool,
    pub smd_and_qc: bool,
    pub compile: bool,
}

impl Default for S2GSteps {
    fn default() -> Self {
        Self {
            decompile: true,
            vtf: true,
            bmp: true,
            smd_and_qc: true,
            compile: true,
        }
    }
}

pub struct S2GBuilder {
    pub settings: S2GSettings,
    path: PathBuf,
    steps: S2GSteps,
    options: S2GOptions,
    process_sync: Option<S2GSync>,
}

impl S2GBuilder {
    #[allow(dead_code)]
    pub fn new(path: &str) -> Self {
        Self {
            settings: S2GSettings::default(),
            path: PathBuf::from(path),
            steps: S2GSteps::default(),
            options: S2GOptions::default(),
            process_sync: None,
        }
    }

    pub fn new_with_path_to_bin(path: &str, path_to_bin: &str) -> Self {
        Self {
            settings: S2GSettings::new(PathBuf::from(path_to_bin).as_path()),
            path: PathBuf::from(path),
            steps: S2GSteps::default(),
            options: S2GOptions::default(),
            process_sync: None,
        }
    }

    pub fn sync(&mut self, sync: S2GSync) -> &mut Self {
        self.process_sync = Some(sync);
        self
    }

    /// Decompiles Source model.
    pub fn decompile(&mut self, decompile: bool) -> &mut Self {
        self.steps.decompile = decompile;
        self
    }

    /// Runs no_vtf to convert .vtf to .png.
    pub fn vtf(&mut self, vtf: bool) -> &mut Self {
        self.steps.vtf = vtf;
        self
    }

    /// Converts .png to compliant .bmp
    pub fn bmp(&mut self, bmp: bool) -> &mut Self {
        self.steps.bmp = bmp;
        self
    }

    /// Converts .smd and .qc.
    pub fn smd_and_qc(&mut self, smd_and_qc: bool) -> &mut Self {
        self.steps.smd_and_qc = smd_and_qc;
        self
    }

    /// Compiles the new GoldSrc model.
    pub fn compile(&mut self, compile: bool) -> &mut Self {
        self.steps.compile = compile;
        self
    }

    pub fn flatshade(&mut self, flatshade: bool) -> &mut Self {
        self.options.flatshade = flatshade;
        self
    }

    /// An amateurish way to instrumentation and proper logging.
    fn log_info(&self, what: &str) {
        println!("{}", what);

        if let Some(sync) = &self.process_sync {
            let mut stdout = sync.stdout.lock().unwrap();
            *stdout += "- [INFO] ";
            *stdout += what;
            *stdout += "\n";
        }
    }

    fn log_err(&self, what: &str) {
        println!("{}", what);

        if let Some(sync) = &self.process_sync {
            let mut stdout = sync.stdout.lock().unwrap();
            *stdout += "- [ERROR] ";
            *stdout += what;
            *stdout += "\n";
        }
    }

    /// Continues with the process even if there is error
    pub fn force(&mut self, force: bool) -> &mut Self {
        self.options.force = force;
        self
    }

    /// Adds "_goldsrc" to the end of output model name
    pub fn add_suffix(&mut self, add_suffix: bool) -> &mut Self {
        self.options.add_suffix = add_suffix;
        self
    }

    pub fn ignore_converted(&mut self, ignore_converted: bool) -> &mut Self {
        self.options.ignore_converted = ignore_converted;
        self
    }

    fn work_decompile(&mut self, input_files: &[PathBuf]) -> eyre::Result<()> {
        self.log_info("Decompiling model");

        let res = input_files.par_iter().map(|input_file| {
            let mut err_str = String::new();

            let mut vvd_path = input_file.clone();
            vvd_path.set_extension(VVD_EXTENSION);

            let mut vtx_path = input_file.clone();
            vtx_path.set_extension(VTX_EXTENSION);

            if !vvd_path.exists() {
                err_str += format!("Cannot find VVD file for {}", input_file.display()).as_str();
            }

            if !vtx_path.exists() {
                err_str += format!("Cannot find VTX file for {}", input_file.display()).as_str();
            }

            if !err_str.is_empty() {
                self.log_err(err_str.as_str());
            }

            if !self.options.force && !err_str.is_empty() {
                return Err(eyre!(err_str));
            }

            // TODO make good
            #[cfg(target_os = "windows")]
            let handle = run_crowbar(input_file, &self.settings.crowbar);

            #[cfg(target_os = "linux")]
            let handle = run_crowbar(
                input_file,
                &self.settings.crowbar,
                self.settings.wineprefix.as_ref().unwrap(),
            );

            let _ = handle.join();

            Ok(())
        });

        if res.filter_map(|a| a.err()).count() > 0 {
            return Err(eyre!("Error with running crowbar"));
        }

        Ok(())
    }

    fn work_vtf(&mut self) -> eyre::Result<()> {
        let folder_path = if self.path.is_dir() {
            &self.path
        } else {
            self.path.parent().unwrap()
        };

        self.log_info(format!("Running no_vtf over {}", folder_path.display()).as_str());

        #[cfg(target_os = "windows")]
        let handle = run_no_vtf(folder_path, &self.settings.no_vtf);

        #[cfg(target_os = "linux")]
        let handle = run_no_vtf(folder_path, &self.settings.no_vtf);

        // usually it would just work
        // TODO: do somethign when it doesn't just work
        let _ = handle.join();

        Ok(())
    }

    fn work_bmp(&mut self) -> eyre::Result<()> {
        self.log_info("Converting PNG to BMP");

        let folder_path = if self.path.is_dir() {
            &self.path
        } else {
            self.path.parent().unwrap()
        };

        let png_files = find_files_with_ext_in_folder(folder_path, "png")?;

        self.log_info(format!("Found ({}) texture file(s)", png_files.len()).as_str());

        match png_to_bmp_folder(&png_files) {
            Ok(_) => {}
            Err(err) => {
                let err_str = format!("Problem with converting PNG to BMP: {}", err);

                self.log_err(&err_str);

                if !self.options.force {
                    return Err(eyre!(err_str));
                }
            }
        };

        Ok(())
    }

    // `input_files` is slice of .mdl files
    fn work_smd_and_qc(&mut self, input_files: &[PathBuf]) -> eyre::Result<Vec<PathBuf>> {
        let mut missing_qc: Vec<PathBuf> = vec![];
        let mut qc_paths: Vec<PathBuf> = vec![];
        let mut compile_able_qcs: Vec<PathBuf> = vec![];

        self.log_info("Converting SMD(s) and QC(s)");

        input_files.iter().for_each(|file| {
            let mut probable_qc = file.clone();
            probable_qc.set_extension("qc");

            if !probable_qc.exists() {
                missing_qc.push(probable_qc)
            } else {
                qc_paths.push(probable_qc)
            }
        });

        if !missing_qc.is_empty() {
            let mut err_str = String::new();

            err_str += "Cannot find some correspondings .qc files: \n";

            for missing in missing_qc {
                err_str += &missing.display().to_string();
                err_str += "\n";
            }

            self.log_err(&err_str);

            if !self.options.force {
                return Err(eyre!(err_str));
            }
        }

        // Qc file would be the top level. There is 1 Qc file and it will link to other Smd files.
        let mut goldsrc_qcs: Vec<(PathBuf, Qc)> = vec![];

        // TODO: just a hack and an assumption that everything is in the same folder
        // good assumption but maybe it can be better in ze future
        let texture_folder = if input_files[0].is_file() {
            input_files[0].parent().unwrap()
        } else {
            input_files[0].as_path()
        };

        let textures = find_files_with_ext_in_folder(texture_folder, "bmp");

        if let Err(err) = &textures {
            let err_str = format!("Cannot open texture folder: {}", err);
            self.log_err(err_str.as_str());
        }

        let textures_in_folder: Vec<String> = textures
            .unwrap()
            .iter()
            .map(|path| {
                path.file_name() // full file name because we can be more flexible from then on
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect();

        let mut missing_textures = HashSet::<String>::new();

        for qc_path in qc_paths.iter() {
            let source_qc = Qc::from_file(qc_path.display().to_string().as_str());

            if let Err(err) = &source_qc {
                let err_str = format!("Cannot load QC {}: {}", qc_path.display(), err);

                self.log_err(&err_str);

                if !self.options.force {
                    return Err(eyre!(err_str));
                }
            }

            let source_qc = source_qc.unwrap();
            let mut goldsrc_qc =
                create_goldsrc_base_qc_from_source(&source_qc, qc_path.parent().unwrap());
            let linked_smds = find_linked_smd_path(qc_path.parent().unwrap(), &source_qc);

            if let Err(err) = &linked_smds {
                let err_str = format!("Cannot find linked SMD for {}: {}", qc_path.display(), err);

                self.log_err(&err_str);

                if !self.options.force {
                    return Err(eyre!(err_str));
                }
            }

            let linked_smds = linked_smds.unwrap();

            let mut qc_textures = HashSet::<String>::new();

            // new smd name will be formated as
            // <old smd name><goldsrc suffix><index>.smd
            // eg: old smd name is `what.smd` -> what_goldsrc0.smd
            // if it is sequence then it will only add the goldsrc suffix
            for SmdInfo {
                name: _,
                smd,
                is_body,
                path,
            } in linked_smds.iter()
            {
                let goldsrc_smds = source_smd_to_goldsrc_smd(smd);
                let smd_file_name = path.file_stem().unwrap().to_str().unwrap();

                for (index, smd) in goldsrc_smds.iter().enumerate() {
                    // check for every texture
                    // TODO: make it efficent but this might be on smd side to use map for each texture to avoid doing thousands plus comparisons
                    // have to iterate everything to make sure that we have every missing textures ever
                    smd.triangles.iter().for_each(|tri| {
                        if !textures_in_folder.contains(&tri.material)
                            && !missing_textures.contains(&tri.material)
                        {
                            missing_textures.insert(tri.material.to_string());
                        } else {
                            qc_textures.insert(tri.material.to_string());
                        }
                    });

                    // if there is missing texture then just don't do anything next
                    // also do this same thing for the qc loop.
                    // but at least do it after opening the qc so we can detect for more missing textures.
                    if !self.options.force && !missing_textures.is_empty() {
                        continue;
                    }

                    let smd_path_for_qc = if *is_body {
                        let name = format!("studio{}", index);
                        let smd_path_for_qc = path
                            .with_file_name(format!("{}{}{}", smd_file_name, GOLDSRC_SUFFIX, index))
                            // .with_extension("smd") // do not write the extension
                            ;

                        goldsrc_qc.add_body(
                            name.as_str(),
                            smd_path_for_qc.display().to_string().as_str(),
                            false,
                            None,
                        );

                        smd_path_for_qc
                    } else {
                        let smd_path_for_qc = path
                            .with_file_name(format!("{}{}", smd_file_name, GOLDSRC_SUFFIX))
                            // .with_extension("smd") // do not write the extension
                            ;
                        // TODO do something more than just idle
                        goldsrc_qc.add_sequence(
                            "idle",
                            smd_path_for_qc.display().to_string().as_str(),
                            vec![],
                        );

                        smd_path_for_qc
                    };

                    let smd_path_for_writing = qc_path.parent().unwrap().join(
                        smd_path_for_qc.with_extension("smd"), // now writes extension because it is file
                    );

                    match smd.write(smd_path_for_writing.display().to_string().as_str()) {
                        Ok(_) => {}
                        Err(err) => {
                            let err_str = format!("Cannot write SMD: {}", err);

                            self.log_err(&err_str);

                            if !self.options.force {
                                return Err(eyre!(err_str));
                            }
                        }
                    };
                }
            }

            if !self.options.force && !missing_textures.is_empty() {
                continue;
            }

            // after writing all of the SMD, now it is time to write our QC
            // not only that, we also add the appropriate model name inside the QC
            let goldsrc_qc_path = qc_path
                .with_file_name(format!(
                    "{}{}",
                    qc_path.file_stem().unwrap().to_str().unwrap(),
                    GOLDSRC_SUFFIX
                ))
                .with_extension("qc");
            let goldsrc_mdl_path = goldsrc_qc_path.with_extension("mdl");

            if self.options.add_suffix {
                goldsrc_qc.add_model_name(goldsrc_mdl_path.display().to_string().as_str());
            } else {
                let goldsrc_model_path = goldsrc_qc_path
                    .with_file_name(qc_path.file_stem().unwrap().to_str().unwrap())
                    .with_extension("mdl");
                goldsrc_qc.add_model_name(goldsrc_model_path.display().to_string().as_str());
            };

            if self.options.flatshade {
                for texture in qc_textures {
                    goldsrc_qc.add_texrendermode(texture.as_str(), qc::RenderMode::FlatShade);
                }
            }

            goldsrc_qcs.push((goldsrc_qc_path, goldsrc_qc));
        }

        // no need to short circuit here because the next condition will do that
        if !missing_textures.is_empty() {
            let mut err_str = format!(
                "Missing ({}) textures in QC folder:",
                missing_textures.len()
            );

            for missing_texture in &missing_textures {
                err_str += "\n";
                err_str += missing_texture;
            }

            self.log_err(&err_str)
        }

        if goldsrc_qcs.len() != qc_paths.len() {
            let err_str = format!(
                "Failed to process {}/{} QC files",
                qc_paths.len() - goldsrc_qcs.len(),
                qc_paths.len()
            );

            self.log_err(&err_str);

            if !self.options.force {
                return Err(eyre!(err_str));
            }
        }

        for (path, qc) in goldsrc_qcs.iter() {
            match qc.write(path.display().to_string().as_str()) {
                Ok(()) => {
                    compile_able_qcs.push(path.clone());
                }
                Err(err) => {
                    let err_str = format!("Cannot write QC {}: {}", path.display(), err);

                    self.log_err(&err_str);

                    if !self.options.force {
                        return Err(eyre!(err_str));
                    }
                }
            }
        }

        Ok(compile_able_qcs)
    }

    fn work_compile(&mut self, compile_able_qcs: &[PathBuf]) -> eyre::Result<Vec<PathBuf>> {
        let mut result: Vec<PathBuf> = vec![];
        let mut instr_msg = format!("Compiling {} model(s):", compile_able_qcs.len());

        compile_able_qcs.iter().for_each(|path| {
            instr_msg += "\n";
            instr_msg += path.display().to_string().as_str();
        });

        self.log_info(instr_msg.as_str());

        let res = compile_able_qcs.par_iter().map(|path| {
            #[cfg(target_os = "windows")]
            let res = run_studiomdl(path, &self.settings.studiomdl);

            #[cfg(target_os = "linux")]
            let res = run_studiomdl(
                path,
                &self.settings.studiomdl,
                self.settings.wineprefix.as_ref().unwrap(),
            );

            match res.join() {
                Ok(res) => {
                    let output = res.unwrap();
                    let stdout = from_utf8(&output.stdout).unwrap();

                    let maybe_err = stdout.find(STUDIOMDL_ERROR_PATTERN);

                    if let Some(err_index) = maybe_err {
                        let err = stdout[err_index + STUDIOMDL_ERROR_PATTERN.len()..].to_string();
                        let err_str = format!("Cannot compile {}: {}", path.display(), err.trim());
                        self.log_err(&err_str);

                        return Err(eyre!(err_str));
                    }

                    Ok(())
                }
                Err(_) => {
                    let err_str =
                        "No idea what happens with running studiomdl. Probably just a dream.";

                    self.log_err(err_str);

                    Err(eyre!(err_str))
                }
            }
        });

        let res =
            res.filter_map(|a| a.err())
                .map(|a| a.to_string())
                .reduce(String::new, |mut acc, e| {
                    acc += &e;
                    acc += "\n";
                    acc
                });

        if !res.is_empty() && !self.options.force {
            return Err(eyre!(res));
        }

        let mut goldsrc_mdl_path = compile_able_qcs
            .iter()
            .map(|path| {
                let mut new_path = path.clone();
                new_path.set_extension("mdl");
                new_path
            })
            .collect::<Vec<PathBuf>>();

        result.append(&mut goldsrc_mdl_path);

        Ok(result)
    }

    /// Does all the work.
    ///
    /// Returns the path of converted models .mdl
    pub fn work(&mut self) -> eyre::Result<Vec<PathBuf>> {
        self.log_info("Starting..............");

        self.log_info("Validating input path");
        if self.path.display().to_string().is_empty() {
            let err_str = "Path is empty";

            self.log_err(err_str);

            return Err(eyre!(err_str));
        }

        if self.path.is_file()
            && (self.path.extension().is_none() || (self.path.extension().unwrap() != "mdl"))
        {
            let err_str = format!("Input file {} is not an MDL", self.path.display());

            self.log_err(&err_str);

            if !self.options.force {
                return Err(eyre!(err_str));
            }
        }

        let input_files = if self.path.is_file() {
            self.log_info("Single file conversion");
            vec![self.path.clone()]
        } else {
            self.log_info("Folder conversion");
            let mdls_in_folder = find_files_with_ext_in_folder(&self.path, "mdl")?;

            let res = if !self.options.ignore_converted {
                mdls_in_folder
            } else {
                mdls_in_folder
                    .iter()
                    .filter(|path| {
                        !path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .contains(GOLDSRC_SUFFIX)
                    })
                    .map(|path| path.to_owned())
                    .collect::<Vec<PathBuf>>()
            };

            res
        };

        let mut input_files_log_str = String::from("Detected MDL(s):");
        input_files.iter().for_each(|file| {
            input_files_log_str += "\n";
            input_files_log_str += file.display().to_string().as_str();
        });

        self.log_info(&input_files_log_str);

        #[cfg(target_os = "linux")]
        match self.settings.check_wineprefix() {
            Ok(_) => {
                self.log_info(
                    format!("WINEPREFIX={}", self.settings.wineprefix.as_ref().unwrap()).as_str(),
                );
            }
            Err(err) => {
                self.log_err(err.to_string().as_str());

                return Err(err);
            }
        };

        // TODO: decompile would not keep anything after ward, just know the result that it works
        if self.steps.decompile {
            self.work_decompile(&input_files)?;
        }

        // TODO what the above
        if self.steps.vtf {
            self.work_vtf()?;
        }

        if self.steps.bmp {
            self.work_bmp()?;
        }

        let mut compile_able_qcs: Vec<PathBuf> = vec![];

        if self.steps.smd_and_qc {
            let mut res = self.work_smd_and_qc(&input_files)?;
            compile_able_qcs.append(&mut res);
        }

        let mut result: Vec<PathBuf> = vec![];

        if self.steps.compile {
            let mut res = self.work_compile(&compile_able_qcs)?;
            result.append(&mut res);
        }

        self.log_info("Done");

        Ok(result)
    }
}

#[derive(Debug)]
struct SmdInfo {
    #[allow(dead_code)]
    name: String,
    smd: Smd,
    is_body: bool,
    path: PathBuf,
}

fn find_linked_smd_path(root: &Path, qc: &Qc) -> eyre::Result<Vec<SmdInfo>> {
    let mut res: Vec<SmdInfo> = vec![];

    for command in qc.commands() {
        let (name, smd, is_body) = match command {
            QcCommand::Body(body) => (body.name.clone(), body.mesh.clone(), true),
            QcCommand::Sequence(sequence) => {
                (sequence.name.clone(), sequence.skeletal.clone(), false)
            }
            QcCommand::BodyGroup(BodyGroup { name: _, bodies }) => {
                // TODO maybe more than 1 body will mess this up
                let body = &bodies[0];
                (body.name.clone(), body.mesh.clone(), true)
            }
            _ => continue,
        };

        // the goal is to returned Smd type so here we will try to open those files
        let smd = maybe_add_extension_to_string(smd.as_str(), "smd");
        let smd = fix_backslash(smd.as_str());
        let smd_path = PathBuf::from(smd);
        let smd = Smd::from_file(
            relative_to_less_relative(root, smd_path.as_path())
                .display()
                .to_string()
                .as_str(),
        )?;

        res.push(SmdInfo {
            name,
            smd,
            is_body,
            path: smd_path,
        });
    }

    Ok(res)
}
