use map::Map;

use crate::modules::check_illegal_brush::check_illegal_brush;

use super::Cli;

pub struct CheckIllegalBrush;
impl Cli for CheckIllegalBrush {
    fn name(&self) -> &'static str {
        "illegal_brush"
    }

    // .map file
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() != 1 {
            self.cli_help();
            return;
        }

        let map = Map::from_file(&args[0]).unwrap();

        check_illegal_brush(&map);
    }

    fn cli_help(&self) {
        println!(
            "\
Map compiler does not tell you enough info about illegal brushes. Here it does.

<.map>
"
        )
    }
}
