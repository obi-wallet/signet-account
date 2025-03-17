use crate::user_account::UserAccount;
#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use secret_toolkit::{serialization::Json, storage::Item};

#[cfg(feature = "cosmwasm")]
pub const USER_ACCOUNT_ADDRESS: Item<String> = Item::new("user_account_address");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const USER_ACCOUNT_ADDRESS: Item<String> = Item::new(b"user_account_address");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const USER_ACCOUNT_CODE_HASH: Item<String> = Item::new(b"user_account_code_hash");

#[cfg(feature = "cosmwasm")]
pub const ACCOUNT: Item<UserAccount> = Item::new("account");
#[cfg(feature = "cosmwasm")]
pub const NEXTHASH: Item<String> = Item::new("nexthash");
#[cfg(feature = "cosmwasm")]
pub const PENDING: Item<UserAccount> = Item::new("pending");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const ACCOUNT: Item<UserAccount, Json> = Item::new(b"account");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const NEXTHASH: Item<String> = Item::new(b"nexthash");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const PENDING: Item<UserAccount, Json> = Item::new(b"pending");
