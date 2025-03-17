//! Key generation.

use cggmp_threshold_ecdsa::mpc_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::{Msg, StateMachine};

use mpc_protocol::Parameters;

use crate::gg20::KeyShare;
use crate::gg2020_old::utils::PartySignup;
use crate::gg_2020::state_machine::keygen::{Error, ProtocolMessage};
use crate::round::RoundMsg;

/// A wrapper around Keygen for use by wasm bindings
pub struct KeygenWrapper(Keygen);

impl KeygenWrapper {
    /// Create a key generator.
    pub fn new(
        parameters: Parameters,
        party_signup: PartySignup,
    ) -> Result<KeygenWrapper, Error> {
        let PartySignup { number, uuid } = party_signup;
        let (party_num_int, _uuid) = (number, uuid);
        Ok(KeygenWrapper(Keygen::new(
            party_num_int,
            parameters.threshold,
            parameters.parties,
        )?))
    }

    /// Handle an incoming message.
    pub fn handle_incoming(
        &mut self,
        message: Msg<<Keygen as StateMachine>::MessageBody>,
    ) -> Result<(), Error> {
        self.0.handle_incoming(message)
    }

    /// Proceed to the next round.
    pub fn proceed(
        &mut self,
    ) -> Result<(u16, Vec<RoundMsg<ProtocolMessage>>), Error> {
        self.0.proceed()?;
        let messages = self.0.message_queue().drain(..).collect();
        let round = self.0.current_round();
        Ok((round, RoundMsg::from_round(round, messages)))
    }

    /// Create the key share.
    pub fn create(&mut self) -> Result<KeyShare, Error> {
        let local_key = self.0.pick_output().unwrap()?;
        Ok(local_key.into())
    }
}
