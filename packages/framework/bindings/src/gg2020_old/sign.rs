//! Message signing.

use wasm_bindgen::prelude::*;

use mpc_driver::gg2020_old::sign::SignerInternal;

/// Round-based signing protocol.
#[wasm_bindgen]
pub struct Signer(SignerInternal);

#[wasm_bindgen]
impl Signer {
    /// Create a signer.
    #[wasm_bindgen(constructor)]
    pub fn new(
        index: JsValue,
        participants: JsValue,
        local_key: JsValue,
    ) -> Result<Signer, JsError> {
        Ok(Signer(SignerInternal::new(
            serde_wasm_bindgen::from_value(index)?,
            serde_wasm_bindgen::from_value(participants)?,
            serde_wasm_bindgen::from_value(local_key)?,
        )?))
    }

    /// Handle an incoming message.
    #[wasm_bindgen(js_name = "handleIncoming")]
    pub fn handle_incoming(
        &mut self,
        message: JsValue,
    ) -> Result<(), JsError> {
        self.0.handle_incoming(serde_wasm_bindgen::from_value(
            message,
        )?)?;
        Ok(())
    }

    /// Proceed to the next round.
    pub fn proceed(&mut self) -> Result<JsValue, JsError> {
        match self.0.proceed()? {
            Some(result) => {
                Ok(serde_wasm_bindgen::to_value(&result)?)
            }
            None => Ok(serde_wasm_bindgen::to_value(&false)?),
        }
    }

    /// Returns the completed offline stage if available.
    #[wasm_bindgen(js_name = "completedOfflineStage")]
    pub fn completed_offline_stage(
        &mut self,
    ) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(
            &self.0.completed_offline_stage()?,
        )?)
    }

    /// Generate the completed offline stage and store the result
    /// internally to be used when `create()` is called.
    ///
    /// Return a partial signature that must be sent to the other
    /// signing participants.
    pub fn partial(
        &mut self,
        message: JsValue,
    ) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(
            &self
                .0
                .partial(serde_wasm_bindgen::from_value(message)?)?,
        )?)
    }

    /// Add partial signatures without validating them. Allows multiple partial signatures
    /// to be combined into a single partial signature before sending it to the other participants.
    pub fn add(
        &mut self,
        partials: JsValue,
    ) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(
            &self.0.add(serde_wasm_bindgen::from_value(partials)?)?,
        )?)
    }

    /// Create and verify the signature.
    pub fn create(
        &mut self,
        partials: JsValue,
    ) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(
            &self
                .0
                .create(serde_wasm_bindgen::from_value(partials)?)?,
        )?)
    }
}
