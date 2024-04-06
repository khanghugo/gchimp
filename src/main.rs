use light_scale::LightScale;
use rotate_prop_static::RotatePropStatic;
use texture_scale::TextureScale;
use types::Cli;

mod light_scale;
mod rotate_prop_static;
mod texture_scale;

mod types;

fn main() {
    main_cli();

    // let mut map = Map::new("./examples/surf_raphaello.map");
    // texture_scale::texture_scale(&mut map, 16.);
    // rotate_prop_static::rotate_prop_static(&mut map, Some("remec_lit_model"));
    // light_scale::light_scale(&mut map, (1., 1., 1., 0.25));

    // map.write("./examples/out.map").unwrap();
}

fn main_cli() {
    let modules: &[&dyn Cli] = &[&LightScale, &RotatePropStatic, &TextureScale];

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