use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use mpc_protocol::hex;

/// Session information for a single party.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartySignup {
    /// Unique index for the party.
    pub number: u16,
    /// Session identifier.
    pub uuid: String,
}

/// Compute the address of an uncompressed public key (65 bytes).
pub(crate) fn address(public_key: &Vec<u8>) -> String {
    // Remove the leading 0x04
    let bytes = &public_key[1..];
    let digest = Keccak256::digest(bytes);
    let final_bytes = &digest[12..];
    format!("0x{}", hex::encode(&final_bytes))
}
