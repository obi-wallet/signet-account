macros::cosmwasm_imports!(StdError, Uint256);

use crate::permissioned_address::CoinBalance;
use crate::sourced_coin::SourcedCoins;
use crate::{sources::Source, sources::Sources};

pub fn get_test_sourced_coin(
    denoms: (String, String),
    amount: Uint256,
    reverse: bool,
) -> Result<SourcedCoins, StdError> {
    match denoms.clone() {
        val if val == ("testtokens".to_string(), "uloop".to_string()) => Ok(SourcedCoins {
            coins: vec![CoinBalance {
                amount,
                denom: denoms.1,
                spent_this_inheritance_period: None,
                limit_remaining: Uint256::from(0u128),
            }],
            wrapped_sources: Sources {
                sources: vec![Source {
                    contract_addr: "test conversion localjuno to loop".to_string(),
                    query_msg: format!("converted {} to {}", amount, amount),
                }],
            },
        }),
        val if val
            == (
                "testtokens".to_string(),
                "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
            )
            || val
                == (
                    "uloop".to_string(),
                    "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                        .to_string(),
                ) =>
        {
            let this_amount = if reverse {
                amount.checked_div(Uint256::from(10000u128)).unwrap()
            } else {
                amount.checked_mul(Uint256::from(100u128)).unwrap()
            };
            Ok(SourcedCoins {
                coins: vec![CoinBalance {
                    amount,
                    denom: denoms.1,
                    spent_this_inheritance_period: None,
                    limit_remaining: Uint256::from(0u128),
                }],
                wrapped_sources: Sources {
                    sources: vec![Source {
                        contract_addr: "test conversion loop to dollars".to_string(),
                        query_msg: format!("converted {} to {}", amount, this_amount),
                    }],
                },
            })
        }
        val if val == ("uloop".to_string(), "testtokens".to_string()) => Ok(SourcedCoins {
            coins: vec![CoinBalance {
                amount,
                denom: denoms.1,
                spent_this_inheritance_period: None,
                limit_remaining: Uint256::from(0u128),
            }],
            wrapped_sources: Sources {
                sources: vec![Source {
                    contract_addr: "test conversion loop to localjuno".to_string(),
                    query_msg: format!("converted {} to {}", amount, amount),
                }],
            },
        }),
        val if val
            == (
                "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
                "uloop".to_string(),
            ) =>
        {
            let this_amount = if !reverse {
                amount.checked_div(Uint256::from(10000u128)).unwrap()
            } else {
                amount.checked_div(Uint256::from(100u128)).unwrap()
            };
            Ok(SourcedCoins {
                coins: vec![CoinBalance {
                    amount,
                    denom: denoms.1,
                    spent_this_inheritance_period: None,
                    limit_remaining: Uint256::from(0u128),
                }],
                wrapped_sources: Sources {
                    sources: vec![Source {
                        contract_addr: "test conversion dollars to loop".to_string(),
                        query_msg: format!("converted {} to {}", amount, this_amount),
                    }],
                },
            })
        }
        val if val
            == (
                "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
                "testtokens".to_string(),
            ) =>
        {
            let this_amount = if !reverse {
                amount.checked_mul(Uint256::from(100u128)).unwrap()
            } else {
                amount.checked_div(Uint256::from(10000u128)).unwrap()
            };
            Ok(SourcedCoins {
                coins: vec![CoinBalance {
                    amount: this_amount,
                    denom: denoms.1,
                    spent_this_inheritance_period: None,
                    limit_remaining: Uint256::from(0u128),
                }],
                wrapped_sources: Sources {
                    sources: vec![Source {
                        contract_addr: "test conversion dollars to juno".to_string(),
                        query_msg: format!("converted {} to {}", amount, this_amount),
                    }],
                },
            })
        }
        _ => Err(StdError::generic_err(format!(
            "unexpected unit test swap denoms: {:?} with amount {} and reverse {}",
            denoms, amount, reverse
        ))),
    }
}
