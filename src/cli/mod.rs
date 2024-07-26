use map::Map;

mod check_illegal_brush;
mod check_missing_texture;
mod custom_script;
mod light_scale;
mod map2mdl;
mod rotate_prop_static;
mod s2g;
mod split_model;
mod texture_scale;

pub enum CliRes {
    NoCli,
    Ok,
    Err,
}

pub trait Cli {
    fn name(&self) -> &'static str {
        "my_module"
    }

    /// Each module has to handle the arguments by itself.
    fn cli(&self) -> CliRes {
        CliRes::Ok
    }

    fn cli_help(&self) {
        println!(
            "\
my_module

Some help text
"
        );
    }
}

/// Runs command-line options
///
/// Returns a boolean to indicate whether any CLI actions taken.
pub fn cli() -> CliRes {
    let mut args = std::env::args();

    // No arguments
    if args.len() <= 1 {
        return CliRes::NoCli;
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
        &map2mdl::Map2MdlCli,
        &split_model::SplitModel,
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
            return module.cli();
        }
    }

    // In case nothing fits then prints this again.
    help();

    CliRes::Err
}
