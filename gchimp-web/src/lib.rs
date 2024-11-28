use std::path::Path;

use wasm_bindgen::prelude::*;

use bsp::Bsp;
use gchimp::modules::{
    loop_wave::loop_wave_from_wave_bytes as _loop_wave,
    resmake::{resmake_single_bsp, ResMakeOptions},
};

#[wasm_bindgen]
pub fn loop_wave(wave_bytes: Vec<u8>) -> Result<Vec<u8>, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    match _loop_wave(wave_bytes) {
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

    match resmake_single_bsp(&bsp, bsp_path, None, &ResMakeOptions { wad_check: false }) {
        Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
        Ok(ok) => Ok(ok),
    }
}
