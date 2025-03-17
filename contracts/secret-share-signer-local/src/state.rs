use secret_cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use secret_toolkit::serialization::Bincode2;
use secret_toolkit::storage::{Item, Keymap};

use crate::msg::CompletedOfflineStageParts;
use ec_curve::secp256k1::point::Secp256k1Point;

// stylistically, we should use pub static, but it breaks something
pub const FEE_MANAGER_ADDRESS: Item<String> = Item::new(b"fee_manager_address");
pub const FEE_MANAGER_CODE_HASH: Item<String> = Item::new(b"fee_manager_code_hash");

static PARTICIPANTS_TO_SHARE_BY_USER_ENTRY_ADDR_SUFFIX: Keymap<
    Vec<u8>,
    CompletedOfflineStageParts,
    Bincode2,
> = Keymap::new(b"participants2ShareByUsrEntryAddr");

pub fn get_pubkey(
    storage: &dyn Storage,
    user_entry_addr: &CanonicalAddr,
) -> Option<Secp256k1Point> {
    PARTICIPANTS_TO_SHARE_BY_USER_ENTRY_ADDR_SUFFIX
        .add_suffix(user_entry_addr.as_slice())
        .iter(storage)
        .ok()?
        .next()?
        .ok()
        .map(|(_, share)| share.pubkey)
}

pub fn get_completed_offline_stage(
    storage: &dyn Storage,
    user_entry_addr: &CanonicalAddr,
    participants: &mut Vec<u8>,
) -> Option<CompletedOfflineStageParts> {
    participants.sort();
    PARTICIPANTS_TO_SHARE_BY_USER_ENTRY_ADDR_SUFFIX
        .add_suffix(user_entry_addr.as_slice())
        .get(storage, participants)
}

pub fn insert_completed_offline_stage(
    storage: &mut dyn Storage,
    user_entry_addr: &CanonicalAddr,
    mut participants: Vec<u8>,
    completed_offline_stage: Option<CompletedOfflineStageParts>,
) -> StdResult<()> {
    participants.sort();
    if let Some(completed_offline_stage) = completed_offline_stage {
        PARTICIPANTS_TO_SHARE_BY_USER_ENTRY_ADDR_SUFFIX
            .add_suffix(user_entry_addr.as_slice())
            .insert(storage, &participants, &completed_offline_stage)
    } else {
        PARTICIPANTS_TO_SHARE_BY_USER_ENTRY_ADDR_SUFFIX
            .add_suffix(user_entry_addr.as_slice())
            .remove(storage, &participants)
    }
}
