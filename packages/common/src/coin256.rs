macros::cosmwasm_imports!(Coin, Uint64, Uint128, Uint256);
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, JsonSchema)]
pub struct Coin256 {
    pub denom: String,
    pub amount: Uint256,
}

impl Coin256 {
    /// Make a new `Coin` from a Uint256 and a denomination.
    pub const fn new(amount: Uint256, denomination: String) -> Self {
        Coin256 {
            denom: denomination,
            amount,
        }
    }

    /// Make a new `Coin256` from a cosmwasm std Coin (which uses Uint128)
    pub fn from_coin128(coin: Coin) -> Self {
        Self::new(Uint256::from(coin.amount.u128()), coin.denom)
    }

    /// Make a new `Coin` from a Uint128 and a denomination.
    pub fn from_uint128(amount: Uint128, denomination: String) -> Self {
        Self::new(Uint256::from(amount), denomination)
    }

    /// Make a new `Coin` from a Uint64 and a denomination.
    pub fn from_uint64(amount: Uint64, denomination: String) -> Self {
        Self::new(Uint256::from(amount), denomination)
    }
}
