use std::io::Write;

use zip::{write::SimpleFileOptions, ZipWriter};

pub struct WasmFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

pub fn zip_files(files: Vec<WasmFile>) -> Vec<u8> {
    let mut buf: Vec<u8> = vec![];

    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));

    let zip_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for file in files {
        zip.start_file(file.name, zip_options).unwrap();
        zip.write_all(&file.bytes).unwrap();
    }

    zip.finish().unwrap();

    buf
}
