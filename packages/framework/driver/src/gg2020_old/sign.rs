//! Message signing.

use cggmp_threshold_ecdsa::mpc_ecdsa::gg_2020::state_machine::sign::SignError;
use curv::{
    arithmetic::Converter, elliptic::curves::Secp256k1, BigInt,
};
use std::array::TryFromSliceError;
use std::convert::TryInto;

use round_based::{Msg, StateMachine};

use crate::gg20::Signature;
use crate::gg2020_old::utils;
use crate::gg_2020::party_i::verify;
use crate::gg_2020::state_machine::keygen::LocalKey;
use crate::gg_2020::state_machine::sign::{
    CompletedOfflineStage, Error, OfflineProtocolMessage,
    OfflineStage, PartialSignature, SignManual,
};
use crate::round::RoundMsg;
use thiserror::Error;

/// SignerErrors
#[derive(Debug, Error)]
pub enum SignerError {
    /// SignError
    #[error("{0}")]
    SignError(#[from] SignError),
    /// GG20Error
    #[error("{0}")]
    GG20Error(#[from] Error),
    /// TrySlice
    #[error("{0}")]
    TrySlice(#[from] TryFromSliceError),
    /// ErrCompletedOfflineStage
    #[error("Completed offline stage unavailable, has partial() been called?")]
    ErrCompletedOfflineStage,
    /// VerifyError
    #[error("Failed to verify signature: {0}")]
    VerifyError(String),
}

/// Round-based signing protocol.
pub struct SignerInternal {
    inner: OfflineStage,
    completed: Option<(CompletedOfflineStage, BigInt)>,
}

impl SignerInternal {
    /// Create a signer.
    pub fn new(
        index: u16,
        participants: Vec<u16>,
        local_key: LocalKey<Secp256k1>,
    ) -> Result<SignerInternal, Error> {
        Ok(SignerInternal {
            inner: OfflineStage::new(
                index,
                participants.clone(),
                local_key,
            )?,
            completed: None,
        })
    }

    /// Handle an incoming message.
    pub fn handle_incoming(
        &mut self,
        message: Msg<<OfflineStage as StateMachine>::MessageBody>,
    ) -> Result<(), Error> {
        self.inner.handle_incoming(message)?;
        Ok(())
    }

    /// Proceed to the next round.
    pub fn proceed(
        &mut self,
    ) -> Result<
        Option<(u16, Vec<RoundMsg<OfflineProtocolMessage>>)>,
        Error,
    > {
        if self.inner.wants_to_proceed() {
            self.inner.proceed()?;
            let messages =
                self.inner.message_queue().drain(..).collect();
            let round = self.inner.current_round();
            let messages = RoundMsg::from_round(round, messages);
            Ok(Some((round, messages)))
        } else {
            Ok(None)
        }
    }

    /// Returns the completed offline stage if available.
    pub fn completed_offline_stage(
        &mut self,
    ) -> Result<CompletedOfflineStage, Error> {
        Ok(CompletedOfflineStage::Full(
            self.inner.pick_output().unwrap()?,
        ))
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
        let completed_offline_stage = CompletedOfflineStage::Full(
            self.inner.pick_output().unwrap()?,
        );
        let data = BigInt::from_bytes(&message);
        let (_sign, partial) = SignManual::new(
            data.clone(),
            completed_offline_stage.clone(),
        )?;

        self.completed = Some((completed_offline_stage, data));
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

        let signature = sign
            .complete(&partials)
            .map_err(|e| SignerError::SignError(e))?;
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
