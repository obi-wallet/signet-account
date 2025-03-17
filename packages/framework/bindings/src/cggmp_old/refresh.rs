#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod bindings {
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsError, JsValue};

    use mpc_driver::cggmp_old::refresh::KeyRefreshWrapper;

    #[wasm_bindgen]
    pub struct KeyRefresh(KeyRefreshWrapper);

    #[wasm_bindgen]
    impl KeyRefresh {
        #[wasm_bindgen(constructor)]
        pub fn new(
            parameters: JsValue,
            local_key: JsValue,
            new_party_index: JsValue,
            old_to_new: JsValue,
            current_t: JsValue,
        ) -> Result<KeyRefresh, JsError> {
            Ok(KeyRefresh(KeyRefreshWrapper::new(
                serde_wasm_bindgen::from_value(parameters)?,
                serde_wasm_bindgen::from_value(local_key)?,
                serde_wasm_bindgen::from_value(new_party_index)?,
                serde_wasm_bindgen::from_value(old_to_new)?,
                serde_wasm_bindgen::from_value(current_t)?,
            )?))
        }

        /// Handle an incoming message.
        #[wasm_bindgen(js_name = "handleIncoming")]
        pub fn handle_incoming(
            &mut self,
            message: JsValue,
        ) -> Result<(), JsError> {
            self.0.handle_incoming(
                serde_wasm_bindgen::from_value(message)?,
            )?;
            Ok(())
        }

        /// Proceed to the next round.
        pub fn proceed(&mut self) -> Result<JsValue, JsError> {
            Ok(serde_wasm_bindgen::to_value(&self.0.proceed()?)?)
        }

        /// Get the key share.
        pub fn create(&mut self) -> Result<JsValue, JsError> {
            Ok(serde_wasm_bindgen::to_value(&self.0.create()?)?)
        }
    }
}
