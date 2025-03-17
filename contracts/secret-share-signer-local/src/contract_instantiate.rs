use secret_cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult};

use crate::{
    msg::InstantiateMsg,
    state::{FEE_MANAGER_ADDRESS, FEE_MANAGER_CODE_HASH},
};

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    FEE_MANAGER_ADDRESS.save(deps.storage, &msg.fee_manager_address)?;
    FEE_MANAGER_CODE_HASH.save(deps.storage, &msg.fee_manager_code_hash)?;
    Ok(Response::default())
}
