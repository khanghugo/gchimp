use std::{
    fs::{self, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

use dem::{demo_writer::DemoWriter, open_demo_from_bytes};

use eyre::eyre;
use rayon::prelude::*;

// yes mean yes doctored, no mean not doctored
pub fn check_doctored(
    demo_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<(PathBuf, bool)> {
    let mut in_bytes = vec![];

    let mut file = OpenOptions::new().read(true).open(demo_path.as_ref())?;
    file.read_to_end(&mut in_bytes)?;

    let demo = open_demo_from_bytes(&in_bytes)?;

    let demo_writer = DemoWriter::new("");
    let out_bytes = demo_writer.write_to_bytes(demo);

    Ok((demo_path.into(), in_bytes == out_bytes))
}

// returns the path of doctored demos
pub fn check_doctored_folder(
    folder_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<Vec<PathBuf>> {
    if !folder_path.as_ref().is_dir() {
        return Err(eyre!("is not a folder"));
    }

    let demo_paths = fs::read_dir(folder_path.as_ref())?
        .filter_map(|read| read.ok())
        .map(|read| read.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().is_some())
        .filter(|path| path.extension().unwrap() == "dem")
        .collect::<Vec<PathBuf>>();

    Ok(demo_paths
        .par_iter()
        .filter_map(|path| check_doctored(path).ok())
        .filter_map(|(path, doctored)| if doctored { Some(path) } else { None })
        .collect())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn not_doctored() {
        let res = check_doctored(
            "/home/khang/bxt/_game_native/cstrike/cg_coldbhop_final_average_benis_0031.20.dem",
        );
        println!("res is {:?}", res.unwrap());
    }

    #[test]
    fn yes_doctored() {
        let res = check_doctored("/home/khang/bxt/_game_native/cstrike/cg_coldbhop_final_average_benis_0031.20_demdoc.dem");
        println!("res is {:?}", res.unwrap());
    }

    #[test]
    fn yes_doctored2() {
        let res = check_doctored("/home/khang/bxt/game_isolated/cstrike/crossfire_demdoc.dem");
        println!("res is {:?}", res.unwrap());
    }

    #[test]
    fn folder() {
        let res = check_doctored_folder("/home/khang/bxt/game_isolated/cstrike/cc1036").unwrap();

        println!("all is not doctoered {:?}", res);
    }
}
