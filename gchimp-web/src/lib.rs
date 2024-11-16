use wasm_bindgen::{convert::FromWasmAbi, prelude::*};

use gchimp::modules::loop_wave::loop_wave_from_wave_bytes as _loop_wave;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn loop_wave(wave_bytes: Vec<u8>) -> Result<Vec<u8>, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    match _loop_wave(wave_bytes) {
        Ok(bytes) => Ok(bytes),
        Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
    }
}
