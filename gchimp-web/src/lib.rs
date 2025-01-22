use std::{path::Path, str::from_utf8};

use smd::Smd;
use utils::{zip_files, WasmFile};
use wasm_bindgen::prelude::*;

use bsp::Bsp;
use gchimp::{
    modules::{
        bsp2wad::bsp2wad_bytes,
        dem2cam::{Dem2CamOptions, _dem2cam_string},
        loop_wave::loop_wave_from_wave_bytes as _loop_wave,
        resmake::{resmake_single_bsp, ResMakeOptions},
    },
    utils::smd_stuffs::maybe_split_smd,
};

mod utils;

#[wasm_bindgen]
pub fn loop_wave(wave_bytes: Vec<u8>, loop_: bool) -> Result<Vec<u8>, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    match _loop_wave(wave_bytes, loop_) {
        Ok(bytes) => Ok(bytes),
        Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
    }
}

#[wasm_bindgen]
pub fn resmake(bsp_bytes: Vec<u8>, filename: &str) -> Result<String, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let bsp = match Bsp::from_bytes(&bsp_bytes) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(JsValue::from_str(
                format!("cannot parse bsp: {}", err).as_str(),
            ))
        }
    };

    let bsp_path = Path::new(filename);

    let options = ResMakeOptions {
        res: true,
        zip: false,
        wad_check: false,
        include_default_resource: false,
        zip_ignore_missing: false,
    };

    // does not include default resource by default
    match resmake_single_bsp(&bsp, bsp_path, None, &options) {
        Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
        Ok(ok) => Ok(ok),
    }
}

#[wasm_bindgen]
pub fn dem2cam(demo_bytes: Vec<u8>, filename: &str, override_fps: f32) -> Result<String, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let demo = dem::open_demo_from_bytes(&demo_bytes);
    let demo = match demo {
        Ok(demo) => demo,
        Err(err) => return Err(JsValue::from_str(err.to_string().as_str())),
    };

    let demo_path = Path::new(filename);

    match _dem2cam_string(
        &demo,
        demo_path,
        &Dem2CamOptions {
            frametime: if override_fps == 0. {
                None
            } else {
                override_fps.into()
            },
            rotation: None,
        },
    ) {
        Ok(ok) => Ok(ok),
        Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
    }
}

#[wasm_bindgen]
pub fn bsp2wad(bsp_bytes: Vec<u8>) -> Result<Vec<u8>, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    match bsp2wad_bytes(&bsp_bytes) {
        Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
        Ok(ok) => Ok(ok),
    }
}

/// `smd_name` should not have .smd suffix for convenience
#[wasm_bindgen]
pub fn split_smd(_smd_string: Vec<u8>, smd_name: String) -> Result<Vec<u8>, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let smd_string = from_utf8(&_smd_string).unwrap();

    let Ok(smd) = Smd::from(&smd_string) else {
        return Err(JsValue::from_str("cannot parse smd"));
    };

    let smds = maybe_split_smd(&smd);

    let files: Vec<WasmFile> = smds
        .into_iter()
        .enumerate()
        .map(|(index, smd)| WasmFile {
            name: format!("{smd_name}_{index}.smd"),
            bytes: smd.write_to_string().unwrap().into_bytes(),
        })
        .collect();

    let bytes = zip_files(files);

    Ok(bytes)
}
