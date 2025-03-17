//! Key refresh
use std::collections::HashMap;

use cggmp_threshold_ecdsa::refresh::state_machine;
use cggmp_threshold_ecdsa::refresh::state_machine::{
    Error, ProtocolMessage,
};
use curv::elliptic::curves::Secp256k1;
use round_based::{Msg, StateMachine};

use mpc_protocol::Parameters;

use crate::gg20::KeyShare;
use crate::gg_2020::state_machine::keygen::LocalKey;
use crate::round::RoundMsg;

/// Key refresh.
pub struct KeyRefreshWrapper(state_machine::KeyRefresh);

impl KeyRefreshWrapper {
    /// Create a key refresh.
    pub fn new(
        parameters: Parameters,
        local_key: Option<LocalKey<Secp256k1>>,
        new_party_index: Option<u16>,
        old_to_new: HashMap<u16, u16>,
        current_t: Option<u16>,
    ) -> Result<KeyRefreshWrapper, Error> {
        Ok(KeyRefreshWrapper(state_machine::KeyRefresh::new(
            local_key,
            new_party_index,
            &old_to_new,
            parameters.threshold,
            parameters.parties,
            current_t,
        )?))
    }

    /// Handle an incoming message.
    pub fn handle_incoming(
        &mut self,
        message: Msg<
            <state_machine::KeyRefresh as StateMachine>::MessageBody,
        >,
    ) -> Result<(), Error> {
        self.0.handle_incoming(message)?;
        Ok(())
    }

    /// Proceed to the next round.
    pub fn proceed(
        &mut self,
    ) -> Result<(u16, Vec<RoundMsg<ProtocolMessage>>), Error> {
        self.0.proceed()?;
        let messages = self.0.message_queue().drain(..).collect();
        let round = self.0.current_round();
        let messages = RoundMsg::from_round(round, messages);
        Ok((round, messages))
    }

    /// Get the key share.
    pub fn create(&mut self) -> Result<KeyShare, Error> {
        let local_key = self.0.pick_output().unwrap()?;
        let key_share: KeyShare = local_key.into();
        Ok(key_share)
    }
}
