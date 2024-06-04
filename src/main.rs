mod cli;
mod gui;
mod modules;
mod utils;

fn main() {
    if !cli::cli() {
        let _ = gui::gui();
    }
}
