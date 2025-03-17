//! Simulation Round-based signing protocol.

use cggmp_threshold_ecdsa::mpc_ecdsa::gg_2020::state_machine::sign::CompletedOfflineStage;
use curv::{
    arithmetic::Converter, elliptic::curves::Secp256k1, BigInt,
};

use crate::gg20::Signature;
use crate::gg2020_old::sign::SignerError;
use crate::gg2020_old::simulate::simulation::Simulation;
use crate::gg2020_old::utils;
use crate::gg_2020::party_i::verify;
use crate::gg_2020::state_machine::keygen::LocalKey;
use crate::gg_2020::state_machine::sign::{
    OfflineStage, PartialSignature, SignManual,
};
use crate::Error;

/// Simulation Signer
pub struct SimulationSignerInternal {
    completed_offline_stage: CompletedOfflineStage,
    completed: Option<(CompletedOfflineStage, BigInt)>,
}

impl SimulationSignerInternal {
    /// Create a signer.
    pub fn new(
        completed_offline_stage: CompletedOfflineStage,
    ) -> SimulationSignerInternal {
        Self {
            completed_offline_stage,
            completed: None,
        }
    }
}

impl SimulationSignerInternal {
    /// Returns the completed offline stage if available.
    pub fn completed_offline_stage(
        &mut self,
    ) -> Result<&CompletedOfflineStage, Error> {
        Ok(&self.completed_offline_stage)
    }

    /// Generate the completed offline stage and store the result
    /// internally to be used when `create()` is called.
    ///
    /// Return a partial signature that must be sent to the other
    /// signing participants.
    pub fn partial(
        &mut self,
        message: Vec<u8>,
    ) -> Result<PartialSignature, SignerError> {
        let message: [u8; 32] = message.as_slice().try_into()?;
        let completed_offline_stage = &self.completed_offline_stage;
        let data = BigInt::from_bytes(&message);
        let (_sign, partial) = SignManual::new(
            data.clone(),
            completed_offline_stage.clone(),
        )?;

        self.completed =
            Some((completed_offline_stage.clone(), data));

        Ok(partial)
    }

    /// Add partial signatures without validating them. Allows multiple partial signatures
    /// to be combined into a single partial signature before sending it to the other participants.
    pub fn add(
        &mut self,
        partials: Vec<PartialSignature>,
    ) -> Result<PartialSignature, SignerError> {
        let (completed_offline_stage, data) =
            self.completed.take().ok_or_else(|| {
                SignerError::ErrCompletedOfflineStage
            })?;
        let (sign, _partial) = SignManual::new(
            data.clone(),
            completed_offline_stage.clone(),
        )?;

        let (_sign, aggregated_partial) = sign.add(&partials)?;

        Ok(aggregated_partial)
    }

    /// Create and verify the signature.
    pub fn create(
        &mut self,
        partials: Vec<PartialSignature>,
    ) -> Result<Signature, SignerError> {
        let (completed_offline_stage, data) =
            self.completed.take().ok_or_else(|| {
                SignerError::ErrCompletedOfflineStage
            })?;
        let pk = match &completed_offline_stage {
            CompletedOfflineStage::Full(f) => {
                f.local_key.y_sum_s.clone()
            }
            CompletedOfflineStage::Minimal(m) => m.pubkey.clone(),
        };

        let (sign, _partial) = SignManual::new(
            data.clone(),
            completed_offline_stage.clone(),
        )?;

        let signature = sign.complete(&partials)?;
        verify(&signature, &pk, &data)
            .map_err(|e| SignerError::VerifyError(e.to_string()))?;

        let public_key = pk.to_bytes(false).to_vec();
        let result = Signature {
            signature,
            address: utils::address(&public_key),
            public_key,
        };

        Ok(result)
    }
}

/// binding to simulate signing's offline stage
pub fn signing_offline_stage_simulated_impl(
    local_keys: Vec<LocalKey<Secp256k1>>,
) -> Result<
    Vec<SimulationSignerInternal>,
    crate::gg_2020::state_machine::sign::Error,
> {
    let mut simulation = Simulation::new();
    let participants: Vec<u16> =
        local_keys.iter().map(|lk| lk.i).collect();
    for (i, local_key) in local_keys.into_iter().enumerate() {
        let offline_stage = OfflineStage::new(
            (i + 1) as u16,
            participants.clone(),
            local_key.clone(),
        )?;
        simulation.add_party(offline_stage);
    }

    simulation.run().map(|stages| {
        stages
            .into_iter()
            .map(|c| {
                SimulationSignerInternal::new(
                    CompletedOfflineStage::Full(c),
                )
            })
            .collect()
    })
}

/// binding to recreate signers from CompletedOfflineStages
pub fn create_signers_impl(
    completed_offline_stages: Vec<CompletedOfflineStage>,
) -> Result<Vec<SimulationSignerInternal>, Error> {
    let simulation_signers = completed_offline_stages
        .into_iter()
        .map(|s| SimulationSignerInternal::new(s))
        .collect::<Vec<_>>();

    Ok(simulation_signers)
}
