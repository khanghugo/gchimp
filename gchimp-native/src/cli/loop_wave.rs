use std::path::PathBuf;

use gchimp::modules::loop_wave::loop_wave;

use super::{Cli, CliRes};

pub struct LoopWave;
impl Cli for LoopWave {
    fn name(&self) -> &'static str {
        "loop_wave"
    }

    // In: path to .wav
    fn cli(&self) -> CliRes {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() != 1 {
            self.cli_help();
            return CliRes::Err;
        }

        let wav_path = PathBuf::from(&args[0]);
        if let Err(err) = loop_wave(wav_path, true) {
            println!("{}", err);
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        println!(
            "\
Makes a .wav loop

<path to .wav>
"
        )
    }
}
