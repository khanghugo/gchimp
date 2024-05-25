mod cli;
mod gui;
mod modules;

fn main() {
    if !cli::cli() {
        let _ = gui::gui();
    }
}
