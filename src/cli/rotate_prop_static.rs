use super::*;

use crate::modules::rotate_prop_static::rotate_prop_static;

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

        let mut map = Map::from_file(&args[0]).unwrap();

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
