#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(
    StdError, StdResult, ensure, Addr, Coin, Deps, Env, Timestamp, Uint128, Uint256
);

use crate::date::{Duration, NaiveDate, NaiveDateTime};
use crate::sourced_coin::SourcedCoins;
use crate::sources::Sources;
use common::coin256::Coin256;
use common::common_error::{ContractError, PermissionedAddressError, ResetError, SpendLimitError};

/// Juno axelar ibc denominator
pub const TERRA_MAINNET_AXLUSDC_IBC: &str =
    "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4";

/// The `PeriodType` type is used for recurring components, including spend limits.
/// Multiples of `Days` and `Months` allow for weekly and yearly recurrence.
#[uniserde::uniserde]
pub enum PeriodType {
    Days,
    Months,
}

#[allow(dead_code)]
enum CheckType {
    TotalLimit,
    RemainingLimit,
}

/// The `CoinBalance` type is a practically extended `Coin` type. Originally intended to
/// including a `limit_remaining` for spend limit cases, but it can now also carry an
/// account balance for beneficiary cases. These two cases may be bifurcated later.
#[uniserde::uniserde]
pub struct CoinBalance {
    /// The denomination. This is be a native asset string as in 'ujuno', a cw20 contract
    /// address, or an 'ibc/...' address
    pub denom: String,
    /// The amount. `to_coin()` will convert this to a `Uint128` as expected for `Coin`
    pub amount: Uint256,
    /// Spent so far from inheritance %.
    pub spent_this_inheritance_period: Option<Uint256>,
    /// For regular spend limits, this is spend limit remaining, as tokens.
    /// For beneficiaries, this is percentage remaining (since limit calculation is a %
    /// of this rather than a ceiling)
    pub limit_remaining: Uint256,
}

impl CoinBalance {
    /// Converts this `CoinBalance` to a cosmwasm_std `Coin`. Drops `current_balance`.
    pub fn to_coin(self) -> Coin256 {
        Coin256::new(self.amount, self.denom)
    }
}

pub trait ConvertibleToCoinBalance {
    fn to_coin_balance(&self, new_amount: Option<Uint256>) -> CoinBalance;
}

impl ConvertibleToCoinBalance for Coin256 {
    /// Converts this `Coin` to a `CoinBalance`. If no `new_amount` is provided, the
    /// `current_balance` will be set to `amount`.
    fn to_coin_balance(&self, new_amount: Option<Uint256>) -> CoinBalance {
        CoinBalance {
            denom: self.denom.clone(),
            amount: self.amount,
            spent_this_inheritance_period: new_amount,
            limit_remaining: self.amount,
        }
    }
}

#[uniserde::uniserde]
pub struct CheckSpendlimitRes {
    pub sourced_coins: SourcedCoins,
    pub should_reset: bool,
    pub as_beneficiary: bool,
}

/// `PermissionedAddressParams` describe the permissions held by a `PermissionedAddress`
/// and can be interpreted as spendlimit (if stored in `params`) or as beneficiary
/// (if stored in `beneficiary_params`).
#[uniserde::uniserde]
pub struct PermissionedAddressParams {
    pub address: String,
    /// `cooldown` holds the current reset time for spend limits if a `PermissionedAddres`.
    /// It holds the main account dormancy threshold if `Beneficiary`.
    pub cooldown: u64,
    ///
    pub period_type: PeriodType,
    pub period_multiple: u16,
    /// Only one spend limit is expected. However, if Beneficiary,
    /// this is taken as a percentage for ANY asset balance, and asset is ignored.
    /// This will be generalized later, but remains this way now to ease contract migration.
    pub spend_limits: Vec<CoinBalance>,
    /// offset of reset time in seconds: 0 means that limits are resetting at 00:00 UTC,
    /// and if monthly, on the 1st. 0 is assumed.
    pub offset: u32,
    /// previously was assumed USDC, but we can name a different asset
    /// (Otherwise axlUSDC is assumed)
    pub denom: Option<String>,
    /// `default` is not really used currently.
    pub default: Option<bool>,
    /// `inheritance_records` store withdrawals of assets for the current period.
    /// Note that the coin.amount here is a percentage withdrawn.
    pub inheritance_records: Vec<Coin256>,
    /// expiration for this rule; 0 is none
    pub expiration: u64,
}

impl Default for PermissionedAddressParams {
    fn default() -> Self {
        Self {
            address: "".to_string(),
            cooldown: 0,
            period_type: PeriodType::Days,
            period_multiple: 0,
            spend_limits: vec![],
            offset: 0,
            denom: None,
            default: None,
            inheritance_records: vec![],
            expiration: 0,
        }
    }
}

impl PermissionedAddressParams {
    /// Named `input_address` as it is only intended for use when a `PermissionedAddressParams`
    /// is coming in as a field in an ExecuteMsg. The `address` field for `PermissionedAddressParams`
    /// is not generally used for querying as the address is stored in the parent `PermissionedAddress`.
    /// (However, there is no known reason why this would differ from the parent address – this
    /// nomenclature is just for additional safety.)
    pub fn input_address(&self) -> String {
        self.address.clone()
    }

    /// Checks whether the `current_time` is past the `current_period_reset` for
    /// this `PermissionedAddress`, which means that the remaining limit CAN be reset to full.
    /// This function does not actually process the reset; use reset_period()
    ///
    /// # Arguments
    ///
    /// * `current_time` - a Timestamp of the current time (or simulated reset time).
    /// Usually `env.block.time`
    pub fn should_reset(&self, current_time: Timestamp) -> bool {
        current_time.seconds() >= self.cooldown
    }

    /// Sets a new reset time for spending limit for this wallet. This also
    /// resets the limit directly by calling self.reset_limits().
    pub fn reset_period(
        &mut self,
        current_time: Timestamp,
    ) -> Result<PermissionedAddressParams, ContractError> {
        let new_dt = NaiveDateTime::from_timestamp(current_time.seconds() as i64, 0u32);
        // how far ahead we set new current_period_reset to
        // depends on the spend limit period (type and multiple)
        let new_dt: Result<NaiveDateTime, ContractError> = match self.period_type {
            // Reset to 0 seconds from midnight, unless there's an offset
            // must allow subtracting a negative so that we add up to offset if necessary
            PeriodType::Days => {
                let working_dt = new_dt
                    .checked_add_signed(Duration::days(self.period_multiple as i64))
                    .and_then(|dt| {
                        dt.checked_sub_signed(Duration::seconds(
                            new_dt.num_seconds_from_midnight() as i64 - self.offset as i64,
                        ))
                    });
                match working_dt {
                    Some(dt) => Ok(dt),
                    None => {
                        return Err(ContractError::Reset(ResetError::DayUpdateError(
                            "unknown error".to_string(),
                        )))
                    }
                }
            }
            // month handling just resets to 00:00:00 on the 1st, so we add offset directly
            PeriodType::Months => {
                let working_month = new_dt.month() as u16 + self.period_multiple;
                match working_month {
                    2..=12 => Ok(NaiveDate::from_ymd(new_dt.year(), working_month as u32, 1)
                        .and_hms(0, 0, 0)),
                    13..=268 => {
                        let year_increment: i32 = (working_month / 12u16) as i32;
                        let dt = NaiveDate::from_ymd(
                            new_dt.year() + year_increment,
                            working_month as u32 % 12,
                            1,
                        )
                        .and_hms(0, 0, 0);
                        let dt = dt.checked_add_signed(Duration::seconds(self.offset as i64));
                        match dt {
                            Some(dt) => Ok(dt),
                            None => {
                                return Err(ContractError::Reset(ResetError::MonthUpdateError {}))
                            }
                        }
                    }
                    _ => Err(ContractError::Reset(ResetError::MonthUpdateError {})),
                }
            }
        };
        self.reset_limits();
        let dt =
            new_dt.map_err(|e| ContractError::Reset(ResetError::DayUpdateError(e.to_string())))?;

        self.cooldown = dt.timestamp() as u64;
        Ok(self.clone())
    }
}

// handlers for modifying spend limits (not reset times)
#[allow(clippy::too_many_arguments)]
impl PermissionedAddressParams {
    pub fn check_spend_vec(
        &self,
        deps: Deps,
        env: Env,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        spend_vec: Vec<CoinBalance>,
        should_reset: bool,
        as_beneficiary: bool,
    ) -> Result<SourcedCoins, ContractError> {
        deps.api.debug(&format!(
            "Getting relevant params. Beneficiary is {}",
            as_beneficiary
        ));
        self.simulate_reduce_limit(
            deps,
            env,
            spend_vec,
            asset_unifier_contract_address,
            #[cfg(feature = "secretwasm")]
            asset_unifier_code_hash,
            should_reset,
            as_beneficiary,
        )
        .map(|tuple| tuple.1)
    }

    /// Returns whether this recurring limit is due for a reset (period has elapsed)
    pub fn get_and_print_should_reset(
        &self,
        deps: Deps,
        current_time: Timestamp,
    ) -> StdResult<bool> {
        if self.should_reset(current_time) {
            deps.api.debug(&format!(
                "\x1b[3m\tRecurring spend limit resets. Current time: {}\x1b[0m",
                current_time.seconds()
            ));
            Ok(true)
        } else {
            deps.api
                .debug("\x1b[3m\tRecurring spend limit does not reset.\x1b[0m");
            Ok(false)
        }
    }

    /// Replaces this wallet's current spending limit. For tests only
    pub fn update_spend_limit(&mut self, new_limit: CoinBalance) -> StdResult<()> {
        self.spend_limits = vec![new_limit];
        Ok(())
    }

    pub fn reset_limits(&mut self) {
        self.spend_limits[0].limit_remaining = self.spend_limits[0].amount;
    }

    /// Simulates a reduction in spend/inheritance limit. Throws an error if
    /// any asset (or total assets) in spend would cause the limit to overflow
    /// negative. Returns the remaining limit - though this is not useful for
    /// beneficiary - and a SourcedCoins object containing the adjustments that
    /// need to be made if this limit is actually reduced in an Execute context.
    #[allow(clippy::too_many_arguments)]
    pub fn simulate_reduce_limit(
        &self,
        deps: Deps,
        env: Env,
        spend: Vec<CoinBalance>,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        reset: bool,
        as_beneficiary: bool,
    ) -> Result<(Uint256, SourcedCoins), ContractError> {
        let unconverted_coins = SourcedCoins {
            coins: spend.clone(),
            wrapped_sources: Sources { sources: vec![] },
        };
        match as_beneficiary {
            true => {
                if !self.spend_limits.is_empty() {
                    deps.api.debug(&format!(
                        "\x1b[3m\tAllowed spend percentage is {}\x1b[0m",
                        self.spend_limits[0].amount
                    ));
                    deps.api.debug(&format!(
                        "\x1b[3m\tCurrent {} balance is {:?}\x1b[0m",
                        spend[0].denom, spend[0].spent_this_inheritance_period
                    ));
                    match self.spend_limits[0].amount {
                        amt if amt == Uint256::from(100u128) => {
                            Ok((Uint256::from(u128::MAX), unconverted_coins))
                        }
                        _ => {
                            // even if reset is true, still needs to return a full
                            // list of the expected reductions
                            let mut limit_reductions = SourcedCoins {
                                coins: vec![],
                                wrapped_sources: Sources { sources: vec![] },
                            };
                            for this_coin in spend {
                                let remaining = self
                                    .inheritance_records
                                    .clone()
                                    .into_iter()
                                    .find(|record| record.denom == this_coin.denom);
                                let reduction = match remaining {
                                    Some(remaining) => remaining
                                        .amount
                                        .checked_sub(
                                            this_coin
                                                .spent_this_inheritance_period
                                                .unwrap()
                                                .checked_mul(self.spend_limits[0].amount)
                                                .unwrap()
                                                .checked_div(Uint256::from(100u128))
                                                .unwrap(),
                                        )
                                        .unwrap(),
                                    None => this_coin
                                        .spent_this_inheritance_period
                                        .unwrap()
                                        .checked_mul(self.spend_limits[0].amount)
                                        .unwrap()
                                        .checked_div(Uint256::from(100u128))
                                        .unwrap(),
                                };
                                ensure!(
                                    this_coin.amount <= reduction,
                                    ContractError::SpendLimit(
                                        SpendLimitError::CannotSpendMoreThanLimit(
                                            this_coin.amount.to_string(),
                                            this_coin.denom
                                        )
                                    )
                                );
                                limit_reductions.coins.push(CoinBalance {
                                    denom: this_coin.denom,
                                    amount: reduction,
                                    spent_this_inheritance_period: None,
                                    limit_remaining: reduction,
                                });
                                // deps.api.debug("Returned limit reductions: {:#?}", limit_reductions);
                            }
                            Ok((Uint256::from(0u128), limit_reductions))
                        }
                    }
                } else {
                    Ok((
                        Uint256::from(0u128),
                        SourcedCoins {
                            coins: vec![],
                            wrapped_sources: Sources { sources: vec![] },
                        },
                    ))
                }
            }
            false => {
                let limit_to_check = match reset {
                    false => self.spend_limits[0].limit_remaining,
                    true => self.spend_limits[0].amount,
                };
                if self.spend_limits[0].denom != unconverted_coins.coins[0].denom {
                    deps.api.debug(&format!(
                        "\x1b[3m\tConverting {:#?} to spendlimit asset\x1b[0m",
                        unconverted_coins.coins[0]
                    ));
                    let converted_spend_amt = if env.block.chain_id == "multitest" {
                        SourcedCoins {
                            coins: unconverted_coins.coins,
                            wrapped_sources: Sources { sources: vec![] },
                        }
                    } else {
                        let sourced_coins = unconverted_coins
                            .convert_to_base_or_target_asset(
                                deps,
                                asset_unifier_contract_address,
                                #[cfg(feature = "secretwasm")]
                                asset_unifier_code_hash,
                                false,
                                env.block.chain_id,
                                Some(self.spend_limits[0].denom.clone()),
                            )
                            .unwrap();
                        SourcedCoins {
                            coins: vec![sourced_coins.asset_unifier.to_coin_balance(None)],
                            wrapped_sources: sourced_coins.sources,
                        }
                    };
                    deps.api.debug(&format!(
                        "\x1b[3m\tReducing limit of {} by {:#?}\x1b[0m",
                        limit_to_check, converted_spend_amt.coins[0]
                    ));
                    let limit_remaining = limit_to_check
                        .checked_sub(converted_spend_amt.coins[0].amount)
                        .map_err(|_| {
                            ContractError::SpendLimit(SpendLimitError::CannotSpendMoreThanLimit(
                                converted_spend_amt.coins[0].amount.to_string(),
                                converted_spend_amt.coins[0].denom.clone(),
                            ))
                        })?;
                    Ok((
                        limit_remaining,
                        SourcedCoins {
                            coins: converted_spend_amt.coins,
                            wrapped_sources: converted_spend_amt.wrapped_sources,
                        },
                    ))
                } else {
                    let limit_remaining = limit_to_check
                        .checked_sub(unconverted_coins.coins[0].amount)
                        .map_err(|_| {
                            ContractError::SpendLimit(SpendLimitError::CannotSpendMoreThanLimit(
                                unconverted_coins.coins[0].amount.to_string(),
                                unconverted_coins.coins[0].denom.clone(),
                            ))
                        })?;
                    Ok((
                        limit_remaining,
                        SourcedCoins {
                            coins: unconverted_coins.coins,
                            wrapped_sources: unconverted_coins.wrapped_sources,
                        },
                    ))
                }
            }
        }
    }

    pub fn process_spend_vec(
        &mut self,
        deps: Deps,
        env: Env,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        spend_vec: Vec<CoinBalance>,
        as_beneficiary: bool,
    ) -> Result<SourcedCoins, ContractError> {
        let all_assets = SourcedCoins {
            coins: spend_vec.clone(),
            wrapped_sources: Sources { sources: vec![] },
        };
        let mut sources = Sources { sources: vec![] };
        let converted_spend = if env.block.chain_id == "multitest" {
            spend_vec[0].clone()
        } else {
            // only works to default base asset at the moment
            let res = all_assets
                .convert_to_base_or_target_asset(
                    deps,
                    asset_unifier_contract_address.clone(),
                    #[cfg(feature = "secretwasm")]
                    asset_unifier_code_hash.clone(),
                    false,
                    env.block.chain_id.clone(),
                    None,
                )
                .unwrap();
            sources = res.sources;
            res.asset_unifier.to_coin_balance(None)
        };
        self.reduce_limit(
            deps,
            env,
            asset_unifier_contract_address,
            #[cfg(feature = "secretwasm")]
            asset_unifier_code_hash,
            converted_spend.clone(),
            as_beneficiary,
        )?;
        Ok(SourcedCoins {
            coins: vec![converted_spend],
            wrapped_sources: sources,
        })
    }

    pub fn reduce_limit(
        &mut self,
        deps: Deps,
        env: Env,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        spend: CoinBalance,
        as_beneficiary: bool,
    ) -> Result<SourcedCoins, ContractError> {
        let spend_limit_reduction: (Uint256, SourcedCoins) = self.simulate_reduce_limit(
            deps,
            env,
            vec![spend],
            asset_unifier_contract_address,
            #[cfg(feature = "secretwasm")]
            asset_unifier_code_hash,
            false,
            as_beneficiary,
        )?;
        self.spend_limits[0].limit_remaining = spend_limit_reduction.0;
        Ok(spend_limit_reduction.1)
    }
}

// functions for tests only
#[cfg(test)]
impl PermissionedAddressParams {
    /// Deprecated, will be axed when better spend limit asset/multiasset
    /// handling is implemented.
    pub fn denom(&self) -> Option<String> {
        self.denom.clone()
    }

    pub fn set_denom(&mut self, new_setting: Option<String>) -> StdResult<()> {
        self.denom = new_setting;
        Ok(())
    }

    pub fn spend_limits(&self) -> Vec<CoinBalance> {
        self.spend_limits.clone()
    }

    pub fn current_period_reset(&self) -> u64 {
        self.cooldown
    }
}

impl PermissionedAddressParams {
    /// Checks whether `addy` can spend `spend` coins.
    #[allow(clippy::too_many_arguments)]
    pub fn check_spendlimits(
        &self,
        deps: Deps,
        env: Env,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        current_time: Timestamp,
        sender: String,
        spend: Vec<CoinBalance>,
        last_activity_time: u64,
        as_beneficiary: bool,
    ) -> Result<CheckSpendlimitRes, ContractError> {
        deps.api
            .debug(format!("Address '{}' attempts to spend", sender).as_str());
        deps.api
            .debug(format!("Last non-beneficiary activity time: {}", last_activity_time).as_str());
        deps.api
            .debug(format!("Current time: {}", current_time).as_str());

        match as_beneficiary {
            false => {
                let coins = self.check_as_spendlimit(
                    deps,
                    env,
                    asset_unifier_contract_address,
                    #[cfg(feature = "secretwasm")]
                    asset_unifier_code_hash,
                    spend,
                    current_time,
                )?;
                Ok(CheckSpendlimitRes {
                    sourced_coins: coins.0,
                    should_reset: coins.1,
                    as_beneficiary: false,
                })
            }
            true => {
                let coins = self.check_as_beneficiary(
                    deps,
                    env,
                    asset_unifier_contract_address,
                    #[cfg(feature = "secretwasm")]
                    asset_unifier_code_hash,
                    spend,
                    current_time,
                    last_activity_time,
                )?;
                Ok(CheckSpendlimitRes {
                    sourced_coins: coins.0,
                    should_reset: coins.1,
                    as_beneficiary: true,
                })
            }
        }
    }

    /// Return the coins for the current account working out the current spendlimit.
    fn check_as_spendlimit(
        &self,
        deps: Deps,
        env: Env,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        spend: Vec<CoinBalance>,
        current_time: Timestamp,
    ) -> Result<(SourcedCoins, bool), ContractError> {
        // should this wallet reset?
        let should_reset_active = self.get_and_print_should_reset(deps, current_time)?;
        // return what the wallet can spend otherwise error
        Ok((
            self.check_spend_vec(
                deps,
                env,
                asset_unifier_contract_address,
                #[cfg(feature = "secretwasm")]
                asset_unifier_code_hash,
                spend,
                should_reset_active,
                false,
            )?,
            should_reset_active,
        ))
    }

    /// Returns the coins of the beneficiary.
    #[allow(clippy::too_many_arguments)]
    fn check_as_beneficiary(
        &self,
        deps: Deps,
        env: Env,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        spend: Vec<CoinBalance>,
        current_time: Timestamp,
        last_activity_time: u64,
    ) -> Result<(SourcedCoins, bool), ContractError> {
        // ensure the beneficiary cooldown has passed and return result
        match self.enforce_beneficiary_cooldown(current_time, last_activity_time) {
            // if the cooldown has passed
            Ok(()) => {
                // check if we should reset the current beneficiary
                let should_reset_beneficiary = self
                    .get_and_print_should_reset(deps, current_time)
                    .map_err(|e| {
                        ContractError::Std(StdError::generic_err(format!(
                            "spendlimit/state.rs:406 {}",
                            e
                        )))
                    })?;
                // return the `SourcedCoins`
                Ok((
                    self.check_spend_vec(
                        deps,
                        env,
                        asset_unifier_contract_address,
                        #[cfg(feature = "secretwasm")]
                        asset_unifier_code_hash,
                        spend,
                        should_reset_beneficiary,
                        true,
                    )?,
                    should_reset_beneficiary,
                ))
            }
            // If there is an error, return error
            Err(f) => Err(f),
        }
    }

    /// Ensures that the specific beneficiary has surpassed its cooldown.
    fn enforce_beneficiary_cooldown(
        &self,
        current_time: Timestamp,
        last_activity_time: u64,
    ) -> Result<(), ContractError> {
        // ensures that the correct time has surpassed
        let multiplier = match self.period_type {
            PeriodType::Days => 86400,
            PeriodType::Months => 2_592_000, // for now, just 30 days for this purpose
        };
        ensure!(
            current_time.seconds() >= last_activity_time + (self.cooldown * multiplier),
            ContractError::PermAddy(PermissionedAddressError::BeneficiaryCooldownNotExpired {})
        );
        // always succeed after successfully checking the cooldown has passed
        Ok(())
    }
}

#[uniserde::uniserde]
pub struct PermissionedAddressesResponse {
    pub permissioned_addresses: Vec<PermissionedAddressParams>,
}

#[cfg(test)]
mod tests {
    use super::*;
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env);

    #[test]
    fn test_process_spend_vec() {
        #[allow(unused_mut)]
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        env.block.chain_id = "multitest".to_string();

        let mut params = PermissionedAddressParams {
            address: "addr1".to_string(),
            cooldown: 0,
            period_type: PeriodType::Days,
            period_multiple: 1,
            spend_limits: vec![CoinBalance {
                denom: "usd".to_string(),
                amount: Uint256::from(1000u128),
                spent_this_inheritance_period: None,
                limit_remaining: Uint256::from(1000u128),
            }],
            offset: 0,
            denom: None,
            default: None,
            inheritance_records: vec![],
            expiration: 0,
        };

        let spend_vec = vec![CoinBalance {
            denom: "usd".to_string(),
            amount: Uint256::from(200u128),
            spent_this_inheritance_period: None,
            limit_remaining: Uint256::from(800u128),
        }];

        // Call process_spend_vec
        let result = params.process_spend_vec(
            deps.as_ref(),
            env,
            "asset_unifier_contract".to_string(),
            #[cfg(feature = "secretwasm")]
            "asset_unifier_code_hash".to_string(),
            spend_vec,
            false,
        );

        // Assertions
        match result {
            Ok(sourced_coin) => {
                assert_eq!(sourced_coin.coins.len(), 1);
                assert_eq!(sourced_coin.coins[0].amount, Uint256::from(200u128));
                assert_eq!(sourced_coin.coins[0].spent_this_inheritance_period, None);
                assert_eq!(
                    sourced_coin.coins[0].limit_remaining,
                    Uint256::from(800u128)
                );
            }
            Err(e) => panic!("process_spend_vec failed: {}", e),
        }

        // Check if the limit_remaining has been updated correctly
        assert_eq!(
            params.spend_limits[0].limit_remaining,
            Uint256::from(800u128)
        );
    }
}
