use std::path::Path;

use map::Map;

use self::{
    custom_script::CustomScript, light_scale::LightScale, rotate_prop_static::RotatePropStatic,
    texture_scale::TextureScale,
};

mod custom_script;
mod light_scale;
mod rotate_prop_static;
mod texture_scale;

pub trait Cli {
    fn name(&self) -> &'static str;
    /// `args[1]` is the name of the function.
    ///
    /// Arguments for the the functions start at `args[2]`
    fn cli(&self);
    fn cli_help(&self);
}

pub fn cli() {
    let modules: &[&dyn Cli] = &[&CustomScript, &LightScale, &RotatePropStatic, &TextureScale];

    let args: Vec<String> = std::env::args().collect();

    let help = || {
        println!(
            "\
map2prop-rs

Available modules:"
        );
        for module in modules {
            println!("{}", module.name());
        }
    };

    if args.len() < 2 {
        help();
        return;
    }

    for module in modules {
        if args[1] == module.name() {
            module.cli();
            return;
        }
    }

    help();
}
