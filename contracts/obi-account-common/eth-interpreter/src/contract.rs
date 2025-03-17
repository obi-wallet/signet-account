// === Imports Start ===
macros::cosmwasm_imports!(
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdResult,
    Uint256,
);
use classes::simulation::{Asset, AssetInfo};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;

use common::{
    common_error::{ContractError, EthError},
    eth::CallData,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, ParseUserOpResponse, QueryMsg};

// === Imports End ===

// === Entry Points Start ===
#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ParseUserOp { user_op } => {
            let calldata = CallData::from_bytes(&user_op.call_data)?;
            match calldata {
                None => Err(ContractError::Eth(EthError::NoCallData {})),
                Some(data) => {
                    let response = ParseUserOpResponse {
                        contract_address: Some(data.contract),
                        spend: vec![Asset {
                            amount: data.amount,
                            info: AssetInfo::Token {
                                contract_addr: "Not implemented".to_string(),
                            },
                        }],
                        fee_recipient: data.fee_recipient,
                        fee: vec![Asset {
                            amount: data.fee_amount.unwrap_or(Uint256::from(0u64)),
                            info: AssetInfo::Token {
                                contract_addr: "Not implemented".to_string(),
                            },
                        }],
                        fields: None,
                        function_signatures: data.function_signatures,
                    };
                    Ok(to_binary(&response)?)
                }
            }
        }
    }
}
// === Entry Points End ===
