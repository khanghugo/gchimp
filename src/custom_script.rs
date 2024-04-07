use std::{fs::OpenOptions, io::Read, path::Path};

use rhai::Engine;

use crate::{light_scale, rotate_prop_static, texture_scale, types::Cli};

fn rotate_prop_static_single(map: &mut map::Map) {
    rotate_prop_static::rotate_prop_static(map, None);
}

fn rotate_prop_static(map: &mut map::Map, new: &str) {
    rotate_prop_static::rotate_prop_static(map, Some(new));
}

fn light_scale(map: &mut map::Map, r: f64, g: f64, b: f64, brightness: f64) {
    light_scale::light_scale(map, (r, g, b, brightness))
}

fn light_scale_int(map: &mut map::Map, r: i64, g: i64, b: i64, brightness: i64) {
    light_scale::light_scale(map, (r as f64, g as f64, b as f64, brightness as f64))
}

fn light_scale_brightness(map: &mut map::Map, brightness: f64) {
    light_scale(map, 1., 1., 1., brightness);
}

fn light_scale_brightness_int(map: &mut map::Map, brightness: i64) {
    light_scale(map, 1., 1., 1., brightness as f64);
}

fn texture_scale(map: &mut map::Map, scalar: f64) {
    texture_scale::texture_scale(map, scalar);
}

fn texture_scale_int(map: &mut map::Map, scalar: i64) {
    texture_scale(map, scalar as f64);
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
        }

        // Rhai engine part
        let mut engine = Engine::new();

        engine
            // light_scale
            .register_fn("light_scale", light_scale)
            .register_fn("light_scale", light_scale_int)
            .register_fn("light_scale", light_scale_brightness)
            .register_fn("light_scale", light_scale_brightness_int)
            // rotate_prop_static
            .register_fn("rotate_prop_static", rotate_prop_static)
            .register_fn("rotate_prop_static", rotate_prop_static_single)
            // texture_scale
            .register_fn("texture_scale", texture_scale)
            .register_fn("texture_scale", texture_scale_int);

        // For write functions. Need to ignore Result.
        engine
            .register_type_with_name::<map::Map>("Map")
            .register_fn("new_map", map::Map::new)
            .register_fn("write", |map, out| {
                let _ = map::Map::write(map, out);
            });

        engine
            .register_type_with_name::<qc::Qc>("Qc")
            .register_fn("new_qc", qc::Qc::new)
            .register_fn("write", |qc, out| {
                let _ = qc::Qc::write(qc, out);
            });

        engine
            .register_type_with_name::<smd::Smd>("Smd")
            .register_fn("new_smd", smd::Smd::new)
            .register_fn("write", |smd, out| {
                let _ = smd::Smd::write(smd, out);
            });

        // Evaluation part
        let rhai_file = &args[0];
        let path = Path::new(&rhai_file);

        let file = OpenOptions::new().read(true).open(path);

        if let Err(err) = file {
            println!("Cannot open file. {}", err);
            return;
        }

        let mut script = String::new();

        if let Err(err) = file.unwrap().read_to_string(&mut script) {
            println!("Cannot read file. {}", err);
            return;
        }

        if let Err(err) = engine.run(&script) {
            println!("Problem with running the script. {}", err);
        };
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
