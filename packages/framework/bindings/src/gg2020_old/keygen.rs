//! Webassembly bindings for the gg2020 key generator.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod bindings {
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsError, JsValue};

    use mpc_driver::gg2020_old::keygen::KeygenWrapper;

    #[wasm_bindgen]
    pub struct KeyGenerator(KeygenWrapper);

    #[wasm_bindgen]
    impl KeyGenerator {
        /// Create a key generator.
        #[wasm_bindgen(constructor)]
        pub fn new(
            parameters: JsValue,
            party_signup: JsValue,
        ) -> Result<KeyGenerator, JsError> {
            let keygen = KeygenWrapper::new(
                serde_wasm_bindgen::from_value(parameters)?,
                serde_wasm_bindgen::from_value(party_signup)?,
            )?;
            Ok(KeyGenerator(keygen))
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

        /// Create the key share.
        pub fn create(&mut self) -> Result<JsValue, JsError> {
            let key_share = self.0.create()?;
            Ok(serde_wasm_bindgen::to_value(&key_share)?)
        }
    }
}
