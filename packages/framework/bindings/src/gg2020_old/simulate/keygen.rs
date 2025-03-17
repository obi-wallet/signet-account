//! Key generation bindings simulated
use mpc_driver::gg2020_old::simulate::keygen::keygen_simulated_impl;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

/// Bindings to simulation based key generation bindings
#[wasm_bindgen(js_name = "keygenSimulated")]
pub fn keygen_simulated(
    parameters: JsValue,
) -> Result<JsValue, JsError> {
    let keyshares = keygen_simulated_impl(
        serde_wasm_bindgen::from_value(parameters)?,
    )?;
    Ok(serde_wasm_bindgen::to_value(&keyshares)?)
}
