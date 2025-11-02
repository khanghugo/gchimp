use std::path::{Path, PathBuf};

use crate::config::parse_config;

use super::*;

pub struct SmdCompile;

impl Cli for SmdCompile {
    fn name(&self) -> &'static str {
        "smd_compile"
    }

    // <.smd file path>
    fn cli(&self) -> CliRes {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.is_empty() {
            self.cli_help();
            return CliRes::Err;
        }

        let smd_paths = args.iter().map(PathBuf::from).collect::<Vec<PathBuf>>();

        if let Err(what) = smd_compile(&smd_paths[0]) {
            println!("{}", what);
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        println!(
            "\
Compiles a .mdl from many .smd

The output will have the same name as the first smd

<.smd> <.smd> <.smd> <.smd> <.smd> ..
"
        )
    }
}

use gchimp::utils::{
    mdl_stuffs::handle_studiomdl_output, run_bin::run_studiomdl, smd_stuffs::maybe_split_smd,
};
use qc::Qc;
use smd::Smd;

pub fn smd_compile(smd_path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
    let idle = Smd::new_basic();
    let mut qc = Qc::new_basic();

    let smd_path = smd_path.as_ref();

    let mut smd = Smd::from_file(smd_path)?;
    let file_name = smd_path.file_stem().unwrap().to_str().unwrap();

    // write out idle file
    idle.write(smd_path.with_file_name("idle.smd"))?;

    // fix smd files to have .bmp suffix
    smd.triangles.iter_mut().for_each(|triangle| {
        if !triangle.material.ends_with(".bmp") {
            triangle.material.push_str(".bmp");
        }
    });

    // split our smd and group with a new file name
    let split_smds = maybe_split_smd(&smd)
        .into_iter()
        .enumerate()
        .map(|(idx, smd)| {
            (
                smd,
                smd_path.with_file_name(format!("{file_name}{idx}.smd")),
            )
        })
        .collect::<Vec<_>>();

    // write our new split smd files
    for (smd, path) in &split_smds {
        smd.write(path)?;
    }

    // do things to qc files
    let smd_path0 = split_smds[0].1.as_path();
    let canon_path = smd_path0.canonicalize().unwrap();

    // this canonical path is used for displaying, not for manipulating
    // on windows, .canonicalize() will add "\\?\"
    // so, we will strip it
    let root_dir = canon_path.parent().unwrap().to_str().unwrap();

    // strip it for root_dir because we have a string
    // only target windows just to make sure
    #[cfg(target_os = "windows")]
    let root_dir = root_dir.strip_prefix(r"\\?\").unwrap_or(root_dir);

    qc.set_model_name(smd_path0.with_extension("mdl").to_str().unwrap());
    qc.set_cd(root_dir);
    qc.set_cd_texture(root_dir);

    split_smds
        .iter()
        .enumerate()
        .for_each(|(idx, (_smd, path))| {
            let stem = path.file_stem().unwrap().to_str().unwrap();

            qc.add_body(format!("studio{idx}").as_str(), stem, false, None);
        });

    qc.add_sequence("idle", "idle", vec![]);

    // write qc file
    let qc_path = smd_path0.with_extension("qc");
    qc.write(qc_path.as_path())?;

    // compiling
    let config = parse_config()?;
    let studiomdl_path = Path::new(&config.studiomdl);

    #[cfg(target_os = "linux")]
    let handle = run_studiomdl(
        &qc_path,
        studiomdl_path,
        config.wineprefix.unwrap().as_str(),
    );

    #[cfg(target_os = "windows")]
    let handle = run_studiomdl(&qc_path, studiomdl_path);

    let handle = handle.join();

    handle_studiomdl_output(handle, None)?;

    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn run() {
        // use std::path::PathBuf;
        // use super::smd_compile;
        // let path1 = PathBuf::from("/home/khang/map/nantu2/res_iter1/landscape.smd");
        // smd_compile(&path1).unwrap();
    }
}
