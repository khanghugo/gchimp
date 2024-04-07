use std::{fs::OpenOptions, io::Read, path::Path};

use rhai::Engine;

use crate::texture_scale::texture_scale;
use crate::{light_scale, rotate_prop_static, types::Cli};
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

        // Rhai engine part
        let mut engine = Engine::new();

        engine
            .register_fn("light_scale", light_scale::light_scale)
            .register_fn("rotate_prop_static", rotate_prop_static::rotate_prop_static)
            .register_fn("texture_scale", texture_scale);

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
Run custom script. Refer to the list of available functions in the source code to better aid yourself.

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
