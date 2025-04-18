use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use cuet::{ChunkWriter, CuePoint};

use crate::err;

pub fn loop_wave(
    wav_path: impl AsRef<Path> + Into<PathBuf>,
    loop_: bool,
    sixteenbit: bool,
) -> eyre::Result<()> {
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

    let bytes = loop_wave_from_wave_bytes(bytes, loop_, sixteenbit)?;

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

fn make_pcm_mono_and_22050(bytes: Vec<u8>, sixteenbit: bool) -> eyre::Result<Vec<u8>> {
    let Ok(mut wav) = wav_io::reader::Reader::from_vec(bytes) else {
        return err!("cannot read .wav bytes");
    };

    let Ok(header) = wav.read_header() else {
        return err!("cannot get .wav header");
    };

    let Ok(f32_samples) = wav.get_samples_f32() else {
        return err!("cannot get .wav samples");
    };

    let f32_samples = match header.channels {
        1 => f32_samples,
        2 => wav_io::utils::stereo_to_mono(f32_samples),
        rest => return err!(".wav has too many channels: {}", rest),
    };

    const BEST_SAMPLING_RATE: u32 = 22050;
    let bit_per_sample = if sixteenbit { 16 } else { 8 };

    let samples2 = wav_io::resample::linear(f32_samples, 1, header.sample_rate, BEST_SAMPLING_RATE);

    let res_wav_header = wav_io::new_header(BEST_SAMPLING_RATE, bit_per_sample, false, true);

    let Ok(res_bytes) = wav_io::write_to_bytes(&res_wav_header, &samples2) else {
        return err!("cannot write .wav");
    };

    Ok(res_bytes)
}

pub fn loop_wave_from_wave_bytes(
    bytes: Vec<u8>,
    loop_: bool,
    sixteenbit: bool,
) -> eyre::Result<Vec<u8>> {
    let mut bytes = make_pcm_mono_and_22050(bytes, sixteenbit)?;

    if loop_ {
        let cue = CuePoint::from_sample_offset(1, 1);
        let cues = [cue];

        let write_cursor = io::Cursor::new(&mut bytes);
        let mut writer = ChunkWriter::new(write_cursor).unwrap();
        writer.append_cue_chunk(cues.as_slice())?;
    }

    Ok(bytes)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn run() {
        let path = PathBuf::from("/home/khang/gchimp/examples/loop_wave/bhit_flesh-1.wav");
        loop_wave(path, true, true).unwrap();
    }

    #[test]
    fn run_32bit() {
        let path = PathBuf::from("/home/khang/gchimp/examples/loop_wave/birds_32bit.wav");
        loop_wave(path, true, true).unwrap();
    }

    #[test]
    fn run_8bit() {
        let path = PathBuf::from("/home/khang/gchimp/examples/loop_wave/eightbits.wav");
        loop_wave(path, true, true).unwrap();
    }
}
