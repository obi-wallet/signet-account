use crate::msg::{CompletedOfflineStageParts, PartialSignature};
use crate::multi_party_ecdsa::local_signature::{LocalSignature, SignatureRecid};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("round 7: {0:?}")]
    Round7(crate::multi_party_ecdsa::Error),
}

pub struct Round7 {
    local_signature: LocalSignature,
}

impl Round7 {
    pub fn new(
        message: &[u8],
        completed_offline_stage: CompletedOfflineStageParts,
    ) -> Result<(Self, PartialSignature), Error> {
        let local_signature = LocalSignature::phase7_local_sig(
            &completed_offline_stage.k_i,
            message,
            &completed_offline_stage.R,
            &completed_offline_stage.sigma_i,
            &completed_offline_stage.pubkey,
        );
        let partial = PartialSignature(local_signature.s_i.clone());
        Ok((Self { local_signature }, partial))
    }

    pub fn proceed_manual(self, sigs: &[PartialSignature]) -> Result<SignatureRecid, Error> {
        let sigs = sigs.iter().map(|s_i| s_i.0.clone()).collect::<Vec<_>>();
        self.local_signature
            .output_signature(&sigs)
            .map_err(Error::Round7)
    }
}
