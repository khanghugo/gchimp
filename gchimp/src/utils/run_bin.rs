use std::path::Path;
use std::{
    process::{Command, Output},
    thread::{self, JoinHandle},
};

#[cfg(target_os = "linux")]
pub fn run_crowbar(
    mdl: &Path,
    crowbar: &Path,
    wineprefix: &str,
) -> JoinHandle<eyre::Result<Output>> {
    // `./crowbar -p model.mdl`
    let command = vec![
        crowbar.display().to_string(),
        "-p".to_string(),
        mdl.display().to_string(),
    ];

    run_command_linux_with_wine(command, wineprefix.to_string())
}

#[cfg(target_os = "windows")]
pub fn run_crowbar(mdl: &Path, crowbar: &Path) -> JoinHandle<eyre::Result<Output>> {
    // `./crowbar -p model.mdl`
    let command = vec![
        crowbar.display().to_string(),
        "-p".to_string(),
        mdl.display().to_string(),
    ];

    run_command_windows(command)
}

#[cfg(target_os = "linux")]
pub fn run_studiomdl(
    qc: &Path,
    studiomdl: &Path,
    wineprefix: &str,
) -> JoinHandle<eyre::Result<Output>> {
    // `./studiomdl file.qc`
    let command = vec![studiomdl.display().to_string(), qc.display().to_string()];
    run_command_linux_with_wine(command, wineprefix.to_string())
}

#[cfg(target_os = "windows")]
pub fn run_studiomdl(qc: &Path, studiomdl: &Path) -> JoinHandle<eyre::Result<Output>> {
    // `./studiomdl file.qc`
    let command = vec![studiomdl.display().to_string(), qc.display().to_string()];
    run_command_windows(command)
}

#[cfg(target_os = "windows")]
pub fn run_command_windows(command: Vec<String>) -> JoinHandle<eyre::Result<Output>> {
    thread::spawn(move || {
        Ok(Command::new(command[0].clone())
            .args(&command[1..])
            .output()?)
    })
}

#[cfg(target_os = "linux")]
pub fn run_command_linux(command: Vec<String>) -> JoinHandle<eyre::Result<Output>> {
    let program = command[0].to_string();
    let args = command[1..].to_vec();

    thread::spawn(move || Ok(Command::new(program).args(args).output()?))
}

#[cfg(target_os = "linux")]
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
