use tiny_keccak::Hasher;

pub mod authorization;
pub mod coin256;
pub mod common;
pub mod common_error;
pub mod common_execute_reasons;
pub mod constants;
pub mod eth;
pub mod legacy_cosmosmsg;
// pub mod osmo_msg;
pub mod universal_msg;

pub fn keccak256hash(bytes: &[u8]) -> String {
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(bytes);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    hex::encode(hash)
}
