//! Key refresh bindings: simulated mode
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod bindings {
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsError, JsValue};

    use mpc_driver::gg2020_old::simulate::refresh::key_refresh_simulated_impl;

    #[wasm_bindgen(js_name = "keyRefreshSimulated")]
    pub fn key_refresh_simulated(
        parameters: JsValue,
        key_refresh_items: JsValue,
    ) -> Result<JsValue, JsError> {
        let updated_key_shares = key_refresh_simulated_impl(
            serde_wasm_bindgen::from_value(parameters)?,
            serde_wasm_bindgen::from_value(key_refresh_items)?,
        )?;
        Ok(serde_wasm_bindgen::to_value(&updated_key_shares)?)
    }
}
