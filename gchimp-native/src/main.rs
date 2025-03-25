#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod cli;
mod config;
mod gui;
mod gui2;
mod persistent_storage;

use std::process::ExitCode;

#[cfg(target_arch = "x86_64")]
fn main() -> ExitCode {
    let cli_res = cli::cli();

    let err_exit = ExitCode::from(1);
    let ok_exit = ExitCode::from(0);

    let a = 1;
    // env -u WAYLAND_DISPLAY cargo run --release

    if a == 1 {
        match cli_res {
            cli::CliRes::NoCli => match gui::gui() {
                Ok(_) => ok_exit,
                Err(_) => err_exit,
            },
            cli::CliRes::Ok => ok_exit,
            cli::CliRes::Err => err_exit,
        }
    } else {
        match cli_res {
            cli::CliRes::NoCli => match gui2::gchimp_native_run() {
                Ok(_) => ok_exit,
                Err(_) => err_exit,
            },
            cli::CliRes::Ok => ok_exit,
            cli::CliRes::Err => err_exit,
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn main() {
    panic!("gchimp-native only works on x86_64 for the time being")
}
