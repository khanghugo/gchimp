use std::thread::{self, JoinHandle};

use super::*;

pub fn maybe_add_extension_to_string(s: &str, ext: &str) -> String {
    let ext_with_dot = format!(".{}", ext);

    if s.ends_with(&ext_with_dot) {
        s.to_string()
    } else {
        format!("{}.{}", s, ext)
    }
}

pub fn find_files_with_ext_in_folder(path: &Path, ext: &str) -> std::io::Result<Vec<PathBuf>> {
    let rd = fs::read_dir(path)?;
    let paths = rd.filter_map(|path| path.ok()).map(|path| path.path());
    let ext_paths = paths
        .filter(|path| path.extension().is_some() && path.extension().unwrap() == ext)
        .collect();

    Ok(ext_paths)
}

pub fn relative_to_less_relative(root: &Path, relative: &Path) -> PathBuf {
    root.join(relative)
}

// TODO: spawn another thread
pub fn run_command_windows(command: Vec<String>) -> JoinHandle<eyre::Result<Output>> {
    thread::spawn(move || Ok(Command::new("cmd").args(command).output()?))
}

pub fn run_command_linux(command: Vec<String>) -> JoinHandle<eyre::Result<Output>> {
    let program = command[0].to_string();
    let args = command[1..].to_vec();

    thread::spawn(move || Ok(Command::new(program).args(args).output()?))
}

pub fn run_command_linux_with_wine(
    command: Vec<String>,
    wine_prefix: String,
) -> JoinHandle<eyre::Result<Output>> {
    thread::spawn(move || {
        Ok(Command::new("wine")
            .args(command)
            .env("WINEPREFIX", wine_prefix)
            .output()?)
    })
}

// i use linux to do things
pub fn fix_backslash(i: &str) -> String {
    i.replace("\\", "/")
}
