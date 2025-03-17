#[cfg(test)]
mod tests {
    macros::cosmwasm_imports!(coins, from_binary, Addr);
    macros::cosmwasm_testing_imports!(
        mock_dependencies,
        mock_dependencies_with_balance,
        mock_env,
        mock_info
    );
    use classes::{gatekeeper_common::InstantiateMsg, msg_gatekeeper_message, rule::Rule};
    use common::{
        authorization::{Authorization, Authorizations},
        eth::EthUserOp,
        universal_msg::UniversalMsg,
    };

    use crate::contract::instantiate;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            eth_interpreter_address: "dummy_address".to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            eth_interpreter_code_hash: "dummy_code_hash".to_string(),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    // TODO: reinstate
    // #[test]
    #[allow(dead_code)]
    fn check_ethereum_message_contract_matches() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            eth_interpreter_address: "dummy_address".to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            eth_interpreter_code_hash: "dummy_code_hash".to_string(),
        };
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let authorized_sender = "carl";

        // dummy sends token contract 5cf29823ccfc73008fa53630d54a424ab82de6f2
        let eth_user_op = EthUserOp::dummy(true, false);
        let auth: Authorization = Authorization {
            identifier: Some(1u16),
            actor: Some(Addr::unchecked(authorized_sender)),
            contract: Some(vec!["5cf29823ccfc73008fa53630d54a424ab82de6f2".to_string()]),
            message_name: None,
            wasmaction_name: None,
            fields: None,
            expiration: 0,
        };
        let msg = msg_gatekeeper_message::QueryMsg::CheckTxAgainstRule {
            msg: UniversalMsg::Eth(eth_user_op),
            sender: authorized_sender.to_string(),
            funds: vec![],
            rule: Rule::Allow(auth),
            user_account: "user_account".to_string(), // unused for eth. todo: option or remove
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: Some("user_account_code_hash".to_string()),
            #[cfg(feature = "cosmwasm")]
            user_account_code_hash: None,
            rule_id: 1,
        };
        let res = crate::contract::query(deps.as_ref(), mock_env(), msg).unwrap();
        let res_de: Authorizations = from_binary(&res).unwrap();
        assert!(res_de.authorizations.len() == 1);
    }
}
