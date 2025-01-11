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

        if let Err(what) = smd_compile(&smd_paths) {
            println!("{}", what);
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        println!(
            "\
Compiles a .mdl from many .smds

The output will have the same name as the first smd

<.smd> <.smd> <.smd> <.smd> <.smd> ..
"
        )
    }
}

use gchimp::{
    err,
    utils::{
        mdl_stuffs::handle_studiomdl_output, run_bin::run_studiomdl, smd_stuffs::maybe_split_smd,
    },
};
use qc::Qc;
use smd::Smd;

pub fn smd_compile(smd_paths: &[PathBuf]) -> eyre::Result<()> {
    let idle = Smd::new_basic();
    let mut qc = Qc::new_basic();

    let smds = smd_paths
        .iter()
        .filter_map(|path| Smd::from_file(path).ok())
        .collect::<Vec<Smd>>();

    if smds.len() != smd_paths.len() {
        return err!("cannot open all smd files");
    }

    // write out idle file
    idle.write(smd_paths[0].with_file_name("idle.smd"))?;

    // fix smd files to have .bmp suffix
    let smds = smds
        .into_iter()
        .map(|mut smd| {
            smd.triangles.iter_mut().for_each(|triangle| {
                if !triangle.material.ends_with(".bmp") {
                    triangle.material.push_str(".bmp");
                }
            });

            smd
        })
        .collect::<Vec<Smd>>();

    // split our smd
    let smds_with_new_name = smds
        .into_iter()
        .zip(smd_paths)
        .flat_map(|(smd, path)| {
            let file_name = path.file_stem().unwrap().to_str().unwrap();

            let smds = maybe_split_smd(&smd);

            smds.into_iter()
                .enumerate()
                .map(|(idx, smd)| (smd, path.with_file_name(format!("{file_name}{idx}.smd"))))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<(Smd, PathBuf)>>();

    // write our new smd again
    for (smd, path) in &smds_with_new_name {
        smd.write(path)?;
    }

    // do things to qc files
    let smd_path0 = smds_with_new_name[0].1.as_path();
    let root_dir = smd_path0.parent().unwrap().to_str().unwrap();

    qc.set_model_name(smd_path0.with_extension("mdl").to_str().unwrap());
    qc.set_cd(root_dir);
    qc.set_cd_texture(root_dir);

    smds_with_new_name
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
    use std::path::PathBuf;

    use super::smd_compile;

    #[test]
    fn run() {
        let path1 = PathBuf::from("/home/khang/map/nantu2/res_iter1/landscape.smd");
        smd_compile(&[path1]).unwrap();
    }
}
