use std::path::Path;

use map::Map;

use crate::modules::{
    custom_script, light_scale::light_scale, rotate_prop_static::rotate_prop_static,
    texture_scale::texture_scale,
};

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

pub struct LightScale;
impl Cli for LightScale {
    fn name(&self) -> &'static str {
        "texture_scale"
    }

    // In, Out, Scale
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 6 {
            self.cli_help();
            return;
        }

        let scalars: Vec<f64> = args
            .iter()
            .skip(2)
            .map(|s| {
                s.parse::<f64>()
                    .map_err(|_| {
                        println!("Cannot parse scalar.");
                        self.cli_help();
                    })
                    .unwrap()
            })
            .collect();

        let mut map = Map::new(&args[0]);

        light_scale(&mut map, (scalars[0], scalars[1], scalars[2], scalars[3]));

        map.write(&args[1]).unwrap();
    }

    fn cli_help(&self) {
        println!(
            "\
light entity _light values scaling.

Multiplying every number in _light field with given scalars

<.map> <output .map> <R> <G> <B> <Brightness>
"
        )
    }
}

pub struct RotatePropStatic;
impl Cli for RotatePropStatic {
    fn name(&self) -> &'static str {
        "rotate_prop_static"
    }

    // In, Out, New name
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 2 {
            self.cli_help();
            return;
        }

        let mut map = Map::new(&args[0]);

        rotate_prop_static(&mut map, if args.len() > 2 { Some(&args[2]) } else { None });

        map.write(&args[1]).unwrap();
    }

    fn cli_help(&self) {
        println!(
            "\
Rotate Source prop_static by +90 Z in (Y Z X)
Can optionally change prop_static to a different entity through classname

<.map> <output .map> <new prop_static classname>
"
        )
    }
}

pub struct TextureScale;
impl Cli for TextureScale {
    fn name(&self) -> &'static str {
        "texture_scale"
    }

    // In, Out, Scale
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 3 {
            self.cli_help();
            return;
        }

        let scalar = args[2].parse::<f64>();

        if scalar.is_err() {
            println!("Cannot parse scalar.");
            self.cli_help();
            return;
        }

        let mut map = Map::new(&args[0]);

        texture_scale(&mut map, scalar.unwrap());

        map.write(&args[1]).unwrap();
    }

    fn cli_help(&self) {
        println!(
            "\
Texture scale

<.map> <output .map> <scalar>
"
        )
    }
}

pub struct CustomScript;
impl Cli for CustomScript {
    fn name(&self) -> &'static str {
        "custom_script"
    }

    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.is_empty() {
            println!("No .rhai file included.");
            self.cli_help();
            return;
        }

        if args[0] == "--help" {
            println!(
                "\
List of functions:

Make sure that the type is consistent. If you have decimal then they all will have at least 1 decimal place.

light_scale(map, brightness)
light_scale(map, r, g, b, brightness)

rotate_prop_static(map)
rotate_prop_static(map, new prop_static name)

texture_scale(map, scalar)

let x = new_map(file_name)
x.write(file_name)

new_qc(file_name) -> x.write(file_name)
new_smd(file_name) -> x.write(file_name)
"
            );

            return;
        }

        let path = Path::new(&args[0]);

        custom_script(path);
    }

    fn cli_help(&self) {
        println!(
            "\
Run custom script. Refer to the list of available functions by having `--help` instead of .rhai file name.

Here is an example.

```example.rhai
let x = new_map(path_to_map);
light_scale(x, (1., 1., 1., 0.5));
x.write(path_to_new_map);
```

MAKE SURE YOU ADD `.` AT THE END OF THE NUMBER WHEN IT IS FLOAT.

\"10 -> 10.\"

<.rhai file>
"
        )
    }
}
