use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = rinf, js_name = send_rust_signal_extern)]
    pub fn send_rust_signal_extern_raw(
        resource: i32,
        message_bytes: Uint8Array,
        binary_included: bool,
        binary_bytes: Uint8Array,
    );
}

pub fn send_rust_signal_extern(
    message_id: i32,
    message_bytes: Vec<u8>,
    binary_included: bool,
    binary_bytes: Vec<u8>,
) {
    send_rust_signal_extern_raw(
        message_id,
        js_sys::Uint8Array::from(message_bytes.as_slice()),
        binary_included,
        js_sys::Uint8Array::from(binary_bytes.as_slice()),
    );
}
