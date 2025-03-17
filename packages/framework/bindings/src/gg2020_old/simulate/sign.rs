//! Message signing bindings: simulated mode
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod bindings {
    use js_sys::Array;
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsError, JsValue};

    use mpc_driver::gg2020_old::simulate::sign::{
        create_signers_impl, signing_offline_stage_simulated_impl,
        SimulationSignerInternal,
    };

    /// Simulation Round-based signing protocol.
    #[wasm_bindgen]
    pub struct SimulationSigner(SimulationSignerInternal);

    #[wasm_bindgen]
    impl SimulationSigner {
        /// Create a signer.
        #[wasm_bindgen(constructor)]
        pub fn new(
            completed_offline_stage: JsValue,
        ) -> Result<SimulationSigner, JsError> {
            Ok(SimulationSigner(SimulationSignerInternal::new(
                serde_wasm_bindgen::from_value(
                    completed_offline_stage,
                )?,
            )))
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
            Ok(serde_wasm_bindgen::to_value(&self.0.partial(
                serde_wasm_bindgen::from_value(message)?,
            )?)?)
        }

        /// Add partial signatures without validating them. Allows multiple partial signatures
        /// to be combined into a single partial signature before sending it to the other participants.
        pub fn add(
            &mut self,
            partials: JsValue,
        ) -> Result<JsValue, JsError> {
            Ok(serde_wasm_bindgen::to_value(
                &self
                    .0
                    .add(serde_wasm_bindgen::from_value(partials)?)?,
            )?)
        }

        /// Create and verify the signature.
        pub fn create(
            &mut self,
            partials: JsValue,
        ) -> Result<JsValue, JsError> {
            Ok(serde_wasm_bindgen::to_value(&self.0.create(
                serde_wasm_bindgen::from_value(partials)?,
            )?)?)
        }
    }

    #[wasm_bindgen(js_name = "signingOfflineStageSimulated")]
    pub fn signing_offline_stage_simulated(
        local_keys: JsValue,
    ) -> Result<Array, JsError> {
        let simulation_signers =
            signing_offline_stage_simulated_impl(
                serde_wasm_bindgen::from_value(local_keys)?,
            )?;
        let signers_result = Array::new();
        for signer in simulation_signers.into_iter() {
            signers_result
                .push(&JsValue::from(SimulationSigner(signer)));
        }

        Ok(signers_result)
    }

    #[wasm_bindgen(js_name = "createSigners")]
    pub fn create_signers(
        completed_offline_stages: JsValue,
    ) -> Result<Array, JsError> {
        let simulation_signers = create_signers_impl(
            serde_wasm_bindgen::from_value(completed_offline_stages)?,
        )?;
        let signers_result = Array::new();
        for signer in simulation_signers.into_iter() {
            signers_result
                .push(&JsValue::from(SimulationSigner(signer)));
        }

        Ok(signers_result)
    }
}
