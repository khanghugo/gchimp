use std::path::Path;

use crate::modules::custom_script::custom_script;

use super::{Cli, CliRes};

pub struct CustomScript;
impl Cli for CustomScript {
    fn name(&self) -> &'static str {
        "custom_script"
    }

    fn cli(&self) -> CliRes {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.is_empty() {
            println!("No .rhai file included.");
            self.cli_help();
            return CliRes::Err;
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

            return CliRes::Err;
        }

        let path = Path::new(&args[0]);

        custom_script(path);

        CliRes::Ok
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
