mod cli;
mod config;
mod gui;
pub mod modules;
pub mod utils;

use std::process::ExitCode;

fn main() -> ExitCode {
    let cli_res = cli::cli();

    let err_exit = ExitCode::from(1);
    let ok_exit = ExitCode::from(0);

    match cli_res {
        cli::CliRes::NoCli => match gui::gui() {
            Ok(_) => ok_exit,
            Err(_) => err_exit,
        },
        cli::CliRes::Ok => ok_exit,
        cli::CliRes::Err => err_exit,
    }
}
