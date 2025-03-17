macros::cosmwasm_imports!(
    ensure,
    to_binary,
    Binary,
    Decimal,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdError,
    StdResult,
    Uint128
);

use crate::state::FEE_PAY_ADDRESSES;
use crate::{msg::ExecuteMsg, state::FEE_DIVISORS, state::LAST_UPDATE};
use common::common_error::AuthorizationError;
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;

use classes::gatekeeper_common::{is_legacy_owner, LEGACY_OWNER};
use common::common_error::ContractError;

use crate::msg::{FeeDetailsResponse, InstantiateMsg, MigrateMsg, QueryMsg};

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    ensure!(
        msg.fee_divisors.1 > 0,
        ContractError::Std(StdError::generic_err("Fee divisor must be greater than 0"))
    );
    #[allow(clippy::declare_interior_mutable_const)]
    #[allow(clippy::borrow_interior_mutable_const)]
    FEE_DIVISORS.insert(deps.storage, &msg.fee_divisors.0, &msg.fee_divisors.1)?;
    #[allow(clippy::declare_interior_mutable_const)]
    #[allow(clippy::borrow_interior_mutable_const)]
    FEE_PAY_ADDRESSES.insert(
        deps.storage,
        &msg.fee_pay_addresses.0,
        &msg.fee_pay_addresses.1,
    )?;
    LEGACY_OWNER.save(deps.storage, &Some(info.sender.to_string()))?;
    LAST_UPDATE.save(deps.storage, &env.block.time.seconds())?;
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetFee {
            chain_id,
            new_fee_divisor,
        } => {
            ensure!(
                new_fee_divisor > 0,
                ContractError::Std(StdError::generic_err("Fee divisor must be greater than 0"))
            );
            ensure!(
                is_legacy_owner(deps.as_ref(), info.sender)?,
                ContractError::Auth(AuthorizationError::Unauthorized(macros::loc_string!()))
            );
            #[allow(clippy::declare_interior_mutable_const)]
            #[allow(clippy::borrow_interior_mutable_const)]
            let stored_fee_divisor = FEE_DIVISORS.get(deps.storage, &chain_id);
            // fee cannot be increased by more than a non-relative 0.1% per 24 hour period
            match stored_fee_divisor {
                None => {
                    LAST_UPDATE.save(deps.storage, &env.block.time.seconds())?;
                    #[allow(clippy::declare_interior_mutable_const)]
                    #[allow(clippy::borrow_interior_mutable_const)]
                    FEE_DIVISORS.insert(deps.storage, &chain_id, &new_fee_divisor)?;
                    Ok(Response::default())
                }
                Some(current_fee_divisor) => {
                    match new_fee_divisor {
                        // fee < old fee means that fee_divisor > current_fee_divisor
                        val if val > current_fee_divisor => {
                            #[allow(clippy::borrow_interior_mutable_const)]
                            let _res = FEE_DIVISORS.remove(deps.storage, &chain_id);
                            #[allow(clippy::declare_interior_mutable_const)]
                            #[allow(clippy::borrow_interior_mutable_const)]
                            FEE_DIVISORS.insert(deps.storage, &chain_id, &new_fee_divisor)?;
                            Ok(Response::default())
                        }
                        val if val == current_fee_divisor => Err(ContractError::Std(
                            StdError::generic_err("Fee divisor is already set to this value"),
                        )),
                        _ => {
                            // Fee_divisor is expressed as divisor, so 1000 = 0.1%, 500 = 0.2%, 333 = 0.3%, and so on.
                            // Here we ensure that no matter what this value, it's not changing more than 0.1% (with
                            // optimistic rounding)
                            ensure!(
                                env.block.time.seconds()
                                    > LAST_UPDATE.load(deps.storage)? + 86_399_u64,
                                ContractError::Std(StdError::generic_err(
                                    "Cannot increase fee more than once per 24 hours"
                                ))
                            );
                            let dividend_dec = Decimal::from_atomics(1u128, 18).map_err(|e| {
                                ContractError::Std(StdError::generic_err(e.to_string()))
                            })?;
                            let old_fee_dec = Decimal::new(current_fee_divisor.into());
                            let new_fee_dec = Decimal::new(new_fee_divisor.into());
                            println!(
                                "checking that {:?} is not more than {:?} greater than {:?}",
                                dividend_dec.checked_div(new_fee_dec),
                                Decimal::from_atomics(1u128, 18),
                                dividend_dec.checked_div(old_fee_dec)
                            );
                            ensure!(
                                dividend_dec
                                    .checked_div(new_fee_dec)
                                    .map_err(|e| ContractError::Std(StdError::generic_err(
                                        e.to_string()
                                    )))?
                                    .checked_sub(dividend_dec.checked_div(old_fee_dec).map_err(
                                        |e| {
                                            ContractError::Std(StdError::generic_err(e.to_string()))
                                        }
                                    )?)?
                                    .floor()
                                    .le(&Decimal::new(1u128.into())),
                                ContractError::Std(StdError::generic_err(
                                    "Cannot increase fee more than 0.1%"
                                ))
                            );
                            LAST_UPDATE.save(deps.storage, &env.block.time.seconds())?;
                            #[allow(clippy::borrow_interior_mutable_const)]
                            FEE_DIVISORS.insert(deps.storage, &chain_id, &new_fee_divisor)?;
                            Ok(Response::default())
                        }
                    }
                }
            }
        }
        ExecuteMsg::SetFeeAddress {
            chain_id,
            new_fee_address,
        } => {
            ensure!(
                is_legacy_owner(deps.as_ref(), info.sender)?,
                ContractError::Auth(AuthorizationError::Unauthorized(macros::loc_string!()))
            );
            #[allow(clippy::declare_interior_mutable_const)]
            #[allow(clippy::borrow_interior_mutable_const)]
            let _res = FEE_PAY_ADDRESSES.remove(deps.storage, &chain_id);
            #[allow(clippy::declare_interior_mutable_const)]
            #[allow(clippy::borrow_interior_mutable_const)]
            FEE_PAY_ADDRESSES.insert(deps.storage, &chain_id, &new_fee_address)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::FeeDetails { chain_id } => {
            #[allow(clippy::borrow_interior_mutable_const)]
            let fee_divisor =
                FEE_DIVISORS
                    .get(deps.storage, &chain_id)
                    .ok_or(StdError::generic_err(format!(
                        "fee_divisor retrieved for chain_id {} is None",
                        chain_id
                    )))?;
            #[allow(clippy::borrow_interior_mutable_const)]
            let fee_pay_address =
                FEE_PAY_ADDRESSES
                    .get(deps.storage, &chain_id)
                    .ok_or(StdError::generic_err(format!(
                        "fee_pay_address retrieved for chain_id {} is None",
                        chain_id
                    )))?;
            let details = FeeDetailsResponse {
                fee_divisor,
                fee_pay_address,
            };
            deps.api.debug(&format!("Got fee details: {:?}", details));
            let details_bin = to_binary(&details)?;
            deps.api.debug(&format!(
                "Converted to binary successfully! Result: {}",
                details_bin
            ));
            Ok(details_bin)
        }
    }
}
