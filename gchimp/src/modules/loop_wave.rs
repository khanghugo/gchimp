use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use cuet::{ChunkWriter, CuePoint};

use crate::err;

pub fn loop_wave(wav_path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
    if !wav_path.as_ref().exists() {
        return err!("{} does not exist", wav_path.as_ref().display());
    }

    if !wav_path.as_ref().is_file() {
        return err!("{} is not a file", wav_path.as_ref().display());
    }

    if wav_path.as_ref().extension().is_none() || wav_path.as_ref().extension().unwrap() != "wav" {
        return err!("{} is not a .wav file", wav_path.as_ref().display());
    }

    let mut file = File::open(wav_path.as_ref()).unwrap();
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;

    let bytes = loop_wave_from_wave_bytes(bytes)?;

    let file_name = wav_path.as_ref().file_stem().unwrap().to_str().unwrap();
    let out_path = wav_path
        .as_ref()
        .with_file_name(format!("{}_loop.wav", file_name));

    let mut out_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    out_file.write_all(&bytes)?;
    out_file.flush()?;

    Ok(())
}

pub fn loop_wave_from_wave_bytes(mut bytes: Vec<u8>) -> eyre::Result<Vec<u8>> {
    let cue = CuePoint::from_sample_offset(1, 1);
    let cues = [cue];

    let write_cursor = io::Cursor::new(&mut bytes);
    let mut writer = ChunkWriter::new(write_cursor).unwrap();
    writer.append_cue_chunk(cues.as_slice())?;

    Ok(bytes)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn run() {
        let path = PathBuf::from("/home/khang/gchimp/examples/loop_wave/bhit_flesh-1.wav");
        loop_wave(path).unwrap();
    }
}
