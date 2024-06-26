use map::Map;

mod check_illegal_brush;
mod check_missing_texture;
mod custom_script;
mod light_scale;
mod rotate_prop_static;
mod s2g;
mod texture_scale;

pub trait Cli {
    fn name(&self) -> &'static str;
    /// Each module has to handle the arguments by itself.
    fn cli(&self);
    fn cli_help(&self);
}

/// Runs command-line options
///
/// Returns a boolean to indicate whether any CLI actions taken.
pub fn cli() -> bool {
    let mut args = std::env::args();

    // No arguments
    if args.len() <= 1 {
        return false;
    }

    // Add new modules here.
    let modules: &[&dyn Cli] = &[
        &custom_script::CustomScript,
        &light_scale::LightScale,
        &rotate_prop_static::RotatePropStatic,
        &texture_scale::TextureScale,
        &s2g::S2G,
        &check_missing_texture::CheckMissingTexture,
        &check_illegal_brush::CheckIllegalBrush,
    ];

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

    // len >= 2
    let command = args.nth(1).unwrap();

    for module in modules {
        if command == module.name() {
            module.cli();
            return true;
        }
    }

    // In case nothing fits then prints this again.
    help();

    true
}
