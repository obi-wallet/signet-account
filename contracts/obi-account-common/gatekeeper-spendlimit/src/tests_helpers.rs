macros::cosmwasm_imports!(Coin, DepsMut, Env, MessageInfo, Uint256);
use classes::gatekeeper_common::CheckTxAgainstRuleResponse;
use classes::msg_gatekeeper_spendlimit::InstantiateMsg;
use common::coin256::Coin256;
use common::common_execute_reasons::{CanExecute, CanExecuteReason};

use classes::rule::Rule;

use crate::contract::can_spend;
use common::common_error::ContractError;
pub const LEGACY_OWNER_STR: &str = "alice";
const OBI_ACCOUNT: &str = "obiaccount";

pub fn get_test_instantiate_message(_env: Env) -> InstantiateMsg {
    InstantiateMsg {
        asset_unifier_contract: "asset_unifier_contract".to_string(),
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        asset_unifier_code_hash: "asset_unifier_code_hash".to_string(),
    }
}

pub fn test_spend_bank(
    deps: DepsMut,
    current_env: Env,
    _to_address: String,
    amount: Vec<Coin256>,
    info: MessageInfo,
    expected_reason: CanExecuteReason,
    rule: Rule,
    rule_id: u16,
) -> Result<CheckTxAgainstRuleResponse, ContractError> {
    let res = can_spend(
        deps.as_ref(),
        current_env,
        OBI_ACCOUNT.to_string(),
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        "dummy_hash".to_string(),
        info.sender.to_string(),
        amount,
        rule,
        rule_id,
    );
    let unwrapped_res = match res {
        Ok(res) => res,
        Err(e) => {
            return Err(e);
        }
    };
    deps.api
        .debug(&format!("unwrapped_res: {:?}", unwrapped_res));
    assert!(unwrapped_res.0.can_execute == CanExecute::Yes(expected_reason));
    Ok(unwrapped_res.0)
}
