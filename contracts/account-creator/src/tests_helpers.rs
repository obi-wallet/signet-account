use std::fmt::Debug;

classes::cosmwasm_imports!(
    to_binary, Addr, BankMsg, Coin, CosmosMsg, StdError, StdResult, Uint128, WasmMsg,
);
use classes::{
    constants::TERRA_MAINNET_AXLUSDC_IBC,
    gatekeeper_common::CheckTxAgainstRuleResponse,
    legacy_cosmosmsg as LegacyMsg,
    signers::{Signer, Signers},
    universal_msg::UniversalMsg,
    user_account::CanExecuteResponse,
};
use cw_multi_test::{App, AppResponse, Executor};
use serde::Serialize;

use crate::tests_setup::{use_contract, CodeIds, ContractAddresses, ObiTestConstants};

pub enum ContractType {
    AccountCreator,
    UserAccount,
    UserEntry,
    SpendlimitGatekeeper,
    MessageGatekeeper,
    UserState,
}

pub fn make_signers(signer_vec: Vec<(&str, &str)>) -> Signers {
    let mut signers: Vec<Signer> = vec![];
    for signer_tuple in signer_vec {
        let signer = Signer {
            address: Addr::unchecked(signer_tuple.0),
            ty: signer_tuple.1.to_string(),
            pubkey_base_64: "".to_string(),
        };
        signers.push(signer);
    }
    Signers::new(signers, None)
}

pub fn check_spend(
    router: &App,
    contract_addresses: ContractAddresses,
    spender: String,
    denom: &str,
    amount: u128,
) -> StdResult<CanExecuteResponse> {
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: spender,
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Send {
            to_address: "bob".to_string(),
            amount: vec![Coin {
                denom: denom.to_string(),
                amount: Uint128::from(amount),
            }],
        })),
    };

    let can_execute_response: StdResult<CanExecuteResponse> = router.wrap().query_wasm_smart(
        use_contract(
            contract_addresses.user_account.clone(),
            contract_addresses,
            "Query".to_string(),
        ),
        &query_msg,
    );

    can_execute_response
}

pub fn check_kobayashi(
    sender: String,
    captain: &str,
    strategy: &str,
    contract_addresses: ContractAddresses,
    router: &App,
) -> StdResult<CanExecuteResponse> {
    let execute_msg = dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
        captain: captain.to_string(),
        strategy: strategy.to_string(),
    };
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: sender,
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
            contract_addr: contract_addresses.dummy_enterprise.to_string(),
            msg: to_binary(&execute_msg).unwrap(),
            funds: vec![],
        })),
        funds: vec![],
    };
    let can_execute_response: StdResult<CanExecuteResponse> = router.wrap().query_wasm_smart(
        use_contract(
            contract_addresses.user_account.clone(),
            contract_addresses,
            "Query".to_string(),
        ),
        &query_msg,
    );
    can_execute_response
}

pub fn factory_deploy(
    router: &mut App,
    legacy_owner: Addr,
    contract_addresses: ContractAddresses,
    _code_ids: CodeIds, //for debug prints
) -> Addr {
    let new_account_msg = classes::account_creator::ExecuteMsg::NewAccount {
        owner: legacy_owner.to_string(),
        signers: make_signers(vec![("signer1", "device"), ("signer2", "phone")]),
        update_delay: 10u64,
        fee_debt: 0u64,
        user_state: None,
        next_hash_seed: "someseed".to_string(),
        user_state_code_hash: None,
    };
    let _res = router
        .execute_contract(
            Addr::unchecked(legacy_owner),
            use_contract(
                contract_addresses.account_creator.as_ref().unwrap().clone(),
                contract_addresses.clone(),
                "Execute".to_string(),
            ),
            &new_account_msg,
            &[],
        )
        .unwrap();
    // hardcoded for now, todo to parse output
    Addr::unchecked("contract14")
}

pub fn execute_test_msg<T>(
    sender: String,
    msg: T,
    contract: ContractType,
    router: &mut App,
    o: &ObiTestConstants,
) -> StdResult<AppResponse>
where
    T: Serialize + Debug,
{
    let contract_address = match contract {
        ContractType::AccountCreator => o.contract_addresses.account_creator.clone().unwrap(),
        ContractType::UserAccount => o.contract_addresses.user_account.clone(),
        ContractType::UserEntry => o.contract_addresses.user_entry.clone(),
        ContractType::SpendlimitGatekeeper => o.contract_addresses.spendlimit_gatekeeper.clone(),
        ContractType::MessageGatekeeper => o.contract_addresses.message_gatekeeper.clone(),
        ContractType::UserState => o.contract_addresses.user_state.clone(),
    };
    let res = router.execute_contract(
        Addr::unchecked(sender),
        use_contract(
            contract_address,
            o.contract_addresses.clone(),
            "Execute".to_string(),
        ),
        &msg,
        &[],
    );
    println!("res: {:#?}", res);
    res.map_err(|e| StdError::GenericErr { msg: e.to_string() })
}
