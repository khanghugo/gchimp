mod cli;
mod config;
mod gui;
pub mod modules;
pub mod utils;

fn main() {
    if !cli::cli() {
        let _ = gui::gui();
    }
}
