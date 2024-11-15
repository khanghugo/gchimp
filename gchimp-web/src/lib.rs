use wasm_bindgen::prelude::*;

use gchimp::modules::loop_wave::loop_wave_from_wave_bytes as _loop_wave;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn loop_wave(wave_bytes: Vec<u8>) -> Vec<u8> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    _loop_wave(wave_bytes).unwrap()
}
