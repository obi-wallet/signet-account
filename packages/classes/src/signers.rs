use common::common_error::{AccountError, ContractError};
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Addr, Deps, Event, StdResult);

/// The `Signer` type identifies a member of the admin multisig, and its type.
/// The format or encryption of type, `ty`, is up to the client.
/// `address` is verified using the deps API when Signer is created.
#[uniserde::uniserde]
pub struct Signer {
    pub address: Addr,
    pub ty: String,
    pub pubkey_base_64: String,
    // arbitrary how this is set up by client
}

#[uniserde::uniserde]
pub struct SignersUnparsed {
    pub signers: Vec<Signer>,
    pub threshold: Option<u8>,
}

#[uniserde::uniserde]
pub struct Signers {
    pub signers: Vec<Signer>,
    pub threshold: u8,
}

impl Signer {
    /// Constructs a new `Signer`. Intended for use in a `Signers`, which is a wrapped
    /// `Vec<Signer>` with its own methods.
    pub fn new(deps: Deps, address: String, ty: String, pubkey_base_64: String) -> StdResult<Self> {
        Ok(Self {
            address: deps.api.addr_validate(&address)?,
            ty,
            pubkey_base_64,
        })
    }

    /// Returns this `Signer`'s type String `ty`.
    pub fn ty(&self) -> String {
        self.ty.clone()
    }

    /// Returns this `Signer`'s `address`, converted to String.
    pub fn address(&self) -> String {
        self.address.to_string()
    }
}

impl Signers {
    /// Returns an `(Event, bool)`, where the `Event` contains all of the signer addresses,
    /// and the `bool` indicates whether a delay should be activated if updating signers.
    /// Intended to make instantiate/confirm_update_admin transactions indexable by signer address.
    ///
    pub fn create_event(&self) -> (Event, bool) {
        let mut activate_delay = false;
        let mut signers_event = Event::new("obisign");
        for signer in self.signers.clone() {
            // this address temporarily hardcoded
            if signer.address.to_string()
                == *"juno17w77rnps59cnallfskg42s3ntnlhrzu2mjkr3e".to_string()
            {
                activate_delay = true;
            }
            signers_event = signers_event.add_attribute("signer", signer.address);
        }
        (signers_event, activate_delay)
    }

    pub fn signers(&self) -> Vec<Signer> {
        self.signers.clone()
    }

    pub fn new(signers: Vec<Signer>, threshold: Option<u8>) -> Result<Self, ContractError> {
        // if threshold is none, the minimum is half the number of signers, rounded down
        // and minus one of course since by convention signatures must be > threshold,
        // not >= threshold
        if signers.is_empty() {
            return Err(ContractError::Account(AccountError::SignersEmpty {}));
        }
        let threshold = threshold
            .unwrap_or((signers.len() as u8) / 2)
            .saturating_sub(1);
        // threshold cannot require more signers than exist
        if threshold >= signers.len() as u8 {
            return Err(ContractError::Account(AccountError::ThresholdTooHigh {}));
        }
        // if signers are > 2, we don't want to require all signers. Really 2-of-2 shouldn't be
        // required either in most setups, but this should not be enforced.
        if signers.len() > 2 && (threshold as usize >= (signers.len() - 1)) {
            return Err(ContractError::Account(AccountError::ThresholdAllSigners {}));
        }
        Ok(Self { signers, threshold })
    }
}
