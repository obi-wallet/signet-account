use common::common_error::ContractError;
#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(feature = "secretwasm")]
use secret_toolkit::storage::Item;

macros::cosmwasm_imports!(ensure, Addr, Coin, StdError, Uint128, Uint256);
use classes::permissioned_address::CoinBalance;

use classes::sourced_coin::SourcedCoins;

use classes::sources::{Source, Sources};
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use uniserde::uniserde;

pub fn get_admin_sourced_coin() -> SourcedCoins {
    SourcedCoins {
        coins: vec![CoinBalance {
            denom: String::from("unlimited"),
            amount: Uint256::from(0u128),
            spent_this_inheritance_period: None,
            limit_remaining: Uint256::from(u128::MAX),
        }],
        wrapped_sources: Sources {
            sources: [Source {
                contract_addr: String::from("no spend limit check"),
                query_msg: String::from("caller is admin"),
            }]
            .to_vec(),
        },
    }
}

#[cfg(feature = "cosmwasm")]
pub const ASSET_UNIFIER_CONTRACT: Item<String> = Item::new("asset_unifier_contract");
#[cfg(feature = "secretwasm")]
pub const ASSET_UNIFIER_CONTRACT: Item<String> = Item::new(b"asset_unifier_contract");
#[cfg(feature = "secretwasm")]
pub const ASSET_UNIFIER_CODE_HASH: Item<String> = Item::new(b"asset_unifier_code_hash");

#[uniserde]
pub struct State {
    pub legacy_owner: Option<String>,
    pub fee_debt: Coin,
    pub immutable_user_account_contract: Addr,
    pub can_update_user_account: bool,
}

impl State {
    /// Converts a `Coin` to an adjustment value.
    ///
    /// If `repaying_debt` is `false`, the returned value will be the positive value of the `Coin`'s amount.
    /// If `repaying_debt` is `true`, the returned value will be the negative value of the `Coin`'s amount.
    ///
    /// # Arguments
    ///
    /// * `coin` - The `Coin` to be converted to an adjustment value.
    /// * `repaying_debt` - A boolean value indicating whether the adjustment is for clearing debt or adding debt.
    /// * `adjustor` - The address of the adjustor.
    ///
    /// # Errors
    ///
    /// Returns an error if the `denom` of `coin` does not match the `denom` of the state's `fee_debt`,
    /// or if the adjustor is not the owner (when adding debt) or the user account contract (when clearing debt).
    pub fn convert_and_validate_adjustment(
        &self,
        coin: Coin,
        repaying_debt: bool,
        adjustor: Addr,
    ) -> Result<i128, ContractError> {
        if self.fee_debt.denom != coin.denom {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "denom mismatch: {} != {}",
                self.fee_debt.denom, coin.denom
            ))));
        }
        match repaying_debt {
            false => {
                // must be human owner OR a signer. For now, owner.
                ensure!(
                    Some(adjustor.to_string()) == self.legacy_owner,
                    ContractError::Std(StdError::generic_err("only owner can add debt"))
                );
                Ok(coin.amount.u128() as i128)
            }
            true => {
                // must not be human owner or signer; MUST be user account contract
                ensure!(
                    adjustor == self.immutable_user_account_contract,
                    ContractError::Std(StdError::generic_err(
                        "only user account contract can clear debt"
                    ))
                );
                Ok(-(coin.amount.u128() as i128))
            }
        }
    }
    // use with negative to clear
    pub fn adjust_debt(&mut self, adjustment: i128) -> Result<Coin, ContractError> {
        let unadjusted_debt: i128 = self.fee_debt.amount.u128() as i128;
        let adjusted_debt: i128 = unadjusted_debt
            .checked_add(adjustment)
            .ok_or_else(|| ContractError::Std(StdError::generic_err("adjustment overflowed")))?;
        let checked_adjusted_debt = match adjusted_debt {
            val if val >= 0 => val as u128,
            _ => {
                return Err(ContractError::Std(StdError::generic_err(
                    "debt cannot be negative",
                )));
            }
        };
        self.fee_debt = Coin {
            denom: self.fee_debt.denom.clone(),
            amount: Uint128::from(checked_adjusted_debt),
        };
        Ok(self.fee_debt.clone())
    }

    /// Returns the current debt.
    pub fn get_debt(&self) -> &Coin {
        &self.fee_debt
    }

    /// Returns the legacy owner of the state.
    /// If there is no owner, returns "no owner".
    pub fn get_legacy_owner(&self) -> String {
        match self.legacy_owner.as_ref() {
            None => String::from("no owner"),
            Some(owner) => owner.to_string(),
        }
    }
}

#[cfg(feature = "cosmwasm")]
pub const STATE: Item<State> = Item::new("state");
#[cfg(feature = "secretwasm")]
pub const STATE: Item<State> = Item::new(b"state");
