macros::cosmwasm_imports!(
    ensure,
    from_binary,
    Binary,
    Deps,
    MessageInfo,
    Response,
    WasmMsg
);
use classes::account_creator::Any;
use common::common_error::{AccountCreatorError, ContractError};

pub fn before_tx(
    _deps: Deps,
    msgs: &[Any],
    _tx_bytes: &Binary,
    _signature: Option<&Binary>,
    _simulate: bool,
) -> Result<Response, ContractError> {
    // no validation here! Anyone can do new_account
    ensure!(
        msgs.len() == 1,
        ContractError::AccountCreator(AccountCreatorError::InvalidMsgsLength {})
    );
    let msg_value: WasmMsg = from_binary(&msgs[0].value)?;
    match msg_value {
        WasmMsg::Execute {
            contract_addr: _,
            code_hash: _,
            msg,
            funds: _,
        } => {
            let msg: classes::account_creator::ExecuteMsg = from_binary(&msg)?;
            match msg {
                classes::account_creator::ExecuteMsg::NewAccount {
                    owner: _,
                    signers: _,
                    fee_debt: _,
                    update_delay: _,
                    user_state: _,
                    user_state_code_hash: _,
                    next_hash_seed: _,
                } => {
                    // proceed!
                }
                _ => {
                    return Err(ContractError::AccountCreator(
                        AccountCreatorError::InvalidMsg {},
                    ))
                }
            }
        }
        _ => {
            return Err(ContractError::AccountCreator(
                AccountCreatorError::InvalidMsg {},
            ))
        }
    }
    Ok(Response::new().add_attribute("method", "before_tx"))
}

pub fn after_tx() -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "after_tx"))
}
