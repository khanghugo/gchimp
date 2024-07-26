use super::*;

use crate::modules::split_model::split_model;

pub struct SplitModel;

impl Cli for SplitModel {
    fn name(&self) -> &'static str {
        "split_model"
    }

    // <.smd file path>
    fn cli(&self) -> CliRes {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() != 1 {
            self.cli_help();
            return CliRes::Err;
        }

        if let Err(err) = split_model(args[0].as_str()) {
            println!("{}", err);
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        println!(
            "\
Split model

The output will have the same name as the input except it will have suffix of indices 

<.qc file>
"
        )
    }
}
