macros::cosmwasm_imports!(
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdResult
);
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{CheaterDetectedResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CHEATER_DETECTED;

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CHEATER_DETECTED.save(deps.storage, &false)?;
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::KobayashiMaru { captain, strategy } => {
            if strategy == *"cheat" {
                CHEATER_DETECTED.save(deps.storage, &true)?;
            }
            let response = Response::new()
                .add_attribute("captain", captain)
                .add_attribute("strategy", strategy);
            Ok(response)
        }
        // for testing deep field
        ExecuteMsg::DeepKobayashiMaru {
            captain,
            strategies,
        } => {
            let response = Response::new()
                .add_attribute("captain", captain)
                .add_attribute("strategy", strategies[0].strategy.clone());
            Ok(response)
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CheaterDetected {} => to_binary(&detect_cheater(deps)?),
    }
}

fn detect_cheater(deps: Deps) -> StdResult<CheaterDetectedResponse> {
    Ok(CheaterDetectedResponse {
        cheater_detected: CHEATER_DETECTED.load(deps.storage)?,
    })
}

#[cfg(test)]
mod tests {}
