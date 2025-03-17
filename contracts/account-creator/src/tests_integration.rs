classes::cosmwasm_imports!(
    coin, to_binary, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, StdError, Timestamp,
    Uint128, Uint256, WasmMsg,
);
use classes::authorization::{FieldComp, KeyValueOptions, StringOrBinary};
use classes::eth::EthUserOp;
use cw1::query;
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{AppResponse, Executor};
use hex;
#[cfg(feature = "cosmwasm")]
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgJoinPool, MsgSwapExactAmountIn};
#[cfg(feature = "cosmwasm")]
use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute;
use serial_test::serial;

#[allow(unused_imports)]
use classes::common_execute_reasons::CanExecuteReason::{
    Allowance, AllowanceAndAllowlist, AllowanceWithAllowlistAndReset,
    AllowanceWithBlanketAuthorizedToken, AllowanceWithReset, Beneficiary, BeneficiaryFullControl,
    BeneficiaryWithAllowlist, BeneficiaryWithAllowlistAndReset, BeneficiaryWithReset,
    NoFundsAndAllowlist, OwnerNoDelay,
};
#[allow(unused_imports)]
use classes::common_execute_reasons::CannotExecuteReason::{
    AllowanceExceeded, AllowanceMessageBlocklisted, BeneficiaryDripExceeded,
    BeneficiaryInheritanceNotActive, BeneficiaryMessageBlocklisted,
    MultipleMessagesNotYetSupported, NoMatchingRule, NoSpendlimitGatekeeper,
};
use classes::common_execute_reasons::{readable, CanExecute, CannotExecuteReason};
use classes::constants::TERRA_MAINNET_AXLUSDC_IBC;
use classes::gatekeeper_common::GatekeeperType;
use classes::user_account::CanExecuteResponse;
use classes::user_state::Rule;
use classes::{
    account_creator::ConfigUpdate,
    authorization::Authorization,
    gatekeeper_common::LegacyOwnerResponse,
    gatekeeper_spendlimit::CanSpendResponse,
    legacy_cosmosmsg as LegacyMsg,
    permissioned_address::{CoinBalance, PeriodType, PermissionedAddressParams},
    universal_msg::UniversalMsg,
    user_account::{GatekeeperContractsResponse, PendingOwnerResponse, SignersResponse},
    user_state::{AbstractionRule, AbstractionRules},
};
use dummy_counter_executable::msg::{CheaterDetectedResponse, Substrategy};

use crate::colors::{GREEN, WHITE, YELLOW_UNDERLINE};
use crate::tests_helpers::{
    check_kobayashi, check_spend, execute_test_msg, make_signers, ContractType,
};
use crate::tests_setup::{mint_native, mock_app, obi_init_tests, use_contract, ObiTestConstants};

fn print_success(msg: &str) {
    println!(
        "{}*** {} ***{}",
        GREEN,
        if msg.is_empty() { "...success" } else { msg },
        WHITE,
    );
    println!();
}

fn print_test_title(msg: &str) {
    println!("{}*** {} ***{}", YELLOW_UNDERLINE, msg, WHITE);
}

#[test]
fn account_factory() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    // mut here since we have to update address after migration
    let mut o: ObiTestConstants =
        obi_init_tests(&mut router, true, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test 0: Deploying new user account using account creator");
    print_success("");

    print_test_title(&format!(
        "Test 1: Updating account config to user account code id {}",
        o.code_ids.user_account
    ));

    let update_config_msg = classes::account_creator::ExecuteMsg::UpdateConfig {
        new_config: ConfigUpdate {
            // Reminder: None here doesn't nullify, just specifies that we're not updating
            // that particular field
            asset_unifier_address: None,
            debt_repay_address: None,
            fee_pay_address: None,
            debtkeeper_code_id: None,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            debtkeeper_code_hash: None,
            default_gatekeepers: None,
            user_account_code_id: Some(o.code_ids.user_account),
            user_state_code_id: None,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            gatekeeper_message_code_hash: None,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            gatekeeper_spendlimit_code_hash: None,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: None,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_state_code_hash: None,
            user_entry_code_id: None,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_entry_code_hash: None,
        },
    };
    execute_test_msg(
        o.obi_owner.to_string(),
        &update_config_msg,
        ContractType::AccountCreator,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    println!("Retrieving gatekeeper addresses...");
    let query_gatekeeper_msg = classes::msg_user_account::QueryMsg::GatekeeperContracts {};
    let res: GatekeeperContractsResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.clone(),
            &query_gatekeeper_msg,
        )
        .unwrap();
    o.contract_addresses.message_gatekeeper = Addr::unchecked(&res.gatekeepers[0]);
    o.contract_addresses.spendlimit_gatekeeper = Addr::unchecked(&res.gatekeepers[1]);

    print_success(&format!("...success. Response: {:#?}", res));
}

#[test]
fn updating_legacy_owner() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test 1a: Non-Owner cannot update legacy owner");
    let update_owner_msg = classes::msg_user_account::ExecuteMsg::ProposeUpdateOwner {
        new_owner: o.new_owner.to_string(),
        signers: make_signers(vec![
            (o.new_owner.as_str(), "default"),
            ("alicesfriend", "recovery"),
        ]),
        signatures: None,
    };
    execute_test_msg(
        o.authorized_spender_daily.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap_err();
    print_success("...error, as expected");

    print_test_title("Test 1b: Owner can start legacy owner update");
    let update_owner_msg = classes::msg_user_account::ExecuteMsg::ProposeUpdateOwner {
        new_owner: o.new_owner.to_string(),
        signers: make_signers(vec![
            (o.new_owner.as_str(), "default"),
            ("alicesfriend", "recovery"),
        ]),
        signatures: None,
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 1c: ...which makes danielle Pending");
    let query_pending_owner = classes::msg_user_account::QueryMsg::PendingOwner {};
    let query_response: PendingOwnerResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_pending_owner,
        )
        .unwrap();
    assert!(query_response.pending_owner == *o.new_owner.to_string());
    print_success("");

    print_test_title("Test 1d: Owner cannot confirm update");
    let update_owner_msg =
        classes::msg_user_account::ExecuteMsg::ConfirmUpdateOwner { signatures: None };
    execute_test_msg(
        o.legacy_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap_err();
    print_success("...error as expected");
    println!();

    print_test_title("Test 1e: But Owner can cancel");
    let update_owner_msg = classes::msg_user_account::ExecuteMsg::CancelUpdateOwner {};
    execute_test_msg(
        o.legacy_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 1f: ...which erases the Pending admin");
    let query_pending_owner = classes::msg_user_account::QueryMsg::PendingOwner {};
    let query_response: PendingOwnerResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_pending_owner,
        )
        .unwrap();
    assert!(query_response.pending_owner == o.legacy_owner);
    print_success("");

    print_test_title("Test 1g: Re-start change to danielle");
    let update_owner_msg = classes::msg_user_account::ExecuteMsg::ProposeUpdateOwner {
        new_owner: o.new_owner.to_string(),
        signers: make_signers(vec![
            (o.new_owner.as_str(), "default"),
            ("alicesfriend", "recovery"),
        ]),
        signatures: None,
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 1h: Confirm the owner update");
    let update_owner_msg =
        classes::msg_user_account::ExecuteMsg::ConfirmUpdateOwner { signatures: None };
    execute_test_msg(
        o.new_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 1i: ...and now danielle is full owner");
    let query_legacy_owner = classes::msg_user_account::QueryMsg::LegacyOwner {};
    let query_response: LegacyOwnerResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_legacy_owner,
        )
        .unwrap();
    assert!(query_response.legacy_owner == o.new_owner.to_string());
    print_success("");

    println!("{}*** Checking signers ***{}", YELLOW_UNDERLINE, WHITE);
    let query_signers = classes::msg_user_account::QueryMsg::Signers {};
    let query_response: SignersResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_signers,
        )
        .unwrap();
    assert!(
        query_response.signers
            == make_signers(vec![
                (o.new_owner.as_str(), "default"),
                ("alicesfriend", "recovery"),
            ])
    );
    print_success("");

    print_test_title("Test 1k: Now change back so tests can continue");
    let update_owner_msg = classes::msg_user_account::ExecuteMsg::ProposeUpdateOwner {
        new_owner: o.legacy_owner.to_string(),
        signers: make_signers(vec![
            (o.legacy_owner.as_str(), "default"),
            ("ownersfriend", "recovery"),
        ]),
        signatures: None,
    };
    execute_test_msg(
        o.new_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    let update_owner_msg =
        classes::msg_user_account::ExecuteMsg::ConfirmUpdateOwner { signatures: None };
    execute_test_msg(
        o.legacy_owner.to_string(),
        &update_owner_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    println!("{}*** And signers ***{}", YELLOW_UNDERLINE, WHITE);
    let query_signers = classes::msg_user_account::QueryMsg::Signers {};
    let query_response: SignersResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_signers,
        )
        .unwrap();
    assert!(
        query_response.signers
            == make_signers(vec![
                (o.legacy_owner.as_str(), "default"),
                ("ownersfriend", "recovery"),
            ])
    );
    print_success("");
}

#[test]
fn spend_limits() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test 2a: Add a permissioned user with a $500 daily spend limit");
    // Let's have bob added as a permissioned user
    let msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_daily.clone(),
                cooldown: o.block_info.time.seconds().checked_add(86400).unwrap(),
                period_type: PeriodType::Days,
                period_multiple: 1,
                offset: 0u32,
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                    amount: Uint256::from(500_000_000u128),
                    limit_remaining: Uint256::from(500_000_000u128),
                    spent_this_inheritance_period: None,
                }],
                denom: Some(TERRA_MAINNET_AXLUSDC_IBC.to_string()),
                default: Some(true),
                inheritance_records: vec![],
            }),
        },
    };
    let contract_type = ContractType::UserAccount;

    execute_test_msg(
        o.legacy_owner.to_string(),
        msg,
        contract_type,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    // Get ID
    let query_msg = classes::msg_user_state::QueryMsg::AbstractionRules {
        actor: None,
        ty: vec![],
    };
    let rules_response: AbstractionRules = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_state.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();

    print_test_title("Test 2b: Upsert same permissioned user down to a $100 daily spend limit");
    // Let's have bob added as a permissioned user
    let msg = classes::msg_user_account::ExecuteMsg::UpsertAbstractionRule {
        id: rules_response.rules[0].id.unwrap(),
        updated_rule: AbstractionRule {
            id: rules_response.rules[0].id,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_daily.clone(),
                cooldown: o.block_info.time.seconds().checked_add(86400).unwrap(),
                period_type: PeriodType::Days,
                period_multiple: 1,
                offset: 0u32,
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                    amount: Uint256::from(500_000_000u128),
                    limit_remaining: Uint256::from(500_000_000u128),
                    spent_this_inheritance_period: None,
                }],
                denom: Some(TERRA_MAINNET_AXLUSDC_IBC.to_string()),
                default: Some(true),
                inheritance_records: vec![],
            }),
        },
    };
    let contract_type = ContractType::UserAccount;

    execute_test_msg(
        o.legacy_owner.to_string(),
        msg,
        contract_type,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    // Query the contract to verify we now have an UPDATED permissioned address
    let query_msg = classes::msg_user_state::QueryMsg::AbstractionRules {
        actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
        ty: vec![GatekeeperType::Spendlimit],
    };
    let abstraction_rules: AbstractionRules = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_state.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    println!(
        "{}*** Matching Abstraction Rules: {} {:#?} ***",
        YELLOW_UNDERLINE, WHITE, abstraction_rules,
    );
    assert_eq!(abstraction_rules.rules.len(), 1);
    if let Rule::Spendlimit(params) = &abstraction_rules.rules[0].main_rule {
        assert_eq!(
            params.spend_limits[0].amount,
            Uint256::from(500_000_000u128)
        );
    } else {
        panic!("Expected Spendlimit rule");
    }

    // we have a $100 USDC spend limit, so we should be able to spend $99...
    // we could query with classes::gatekeeper_spendlimit::QueryMsg::CanSpend,
    // but this is an integration test
    print_test_title("Test 3a: Check that permissioned user can spend $99");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        TERRA_MAINNET_AXLUSDC_IBC,
        99_000_000u128,
    );
    let unwrapped_res = can_execute_response.unwrap();
    assert!(unwrapped_res.can_execute == CanExecute::Yes(Allowance));
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    // let's actually execute the spend so that the spend limit is updated
    /*
     *
     * TODO
     *
     *
     *
     *

    // now we should NOT be able to spend even $2
    print_test_title("Test 3c: Try (and fail) to send $2");
    let can_spend_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        TERRA_MAINNET_AXLUSDC_IBC,
        2_000_000u128,
    )
    .unwrap();
    assert!(can_spend_response.can_execute == CanExecute::No(AllowanceExceeded));
    if let CanExecute::Yes(reason) = can_spend_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();

    // nor can we spend 2 "ujunox"
    print_test_title("Test 3d: Try (and fail) to send 2 Juno (valued by dummy dex at $4.56 each)");
    let can_spend_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        "ujunox",
        2_000_000u128,
    )
    .unwrap();
    assert!(can_spend_response.can_execute == CanExecute::No(AllowanceExceeded));
    if let CanExecute::No(reason) = can_spend_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();
    */

    // but we can spend $1
    print_test_title("Test 3e: Check we can spend $1");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        TERRA_MAINNET_AXLUSDC_IBC,
        1_000_000u128,
    );
    let unwrapped_res = can_execute_response.unwrap();
    assert!(unwrapped_res.can_execute == CanExecute::Yes(Allowance));
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    // or 0.1 JUNO
    print_test_title("Test 3f: Check we can spend 0.1 Juno ($0.45), with repayment");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        "ujunox",
        100_000u128,
    );
    let unwrapped_res = can_execute_response.unwrap();
    assert!(unwrapped_res.can_execute == CanExecute::Yes(Allowance));
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 3i: Go forward 1 day (actually, go to midnight next day, which is <86400 seconds), and now we can spend $2 since limit has reset");
    let old_block_info = router.block_info();
    router.set_block(BlockInfo {
        height: 12345u64, // used sometimes to detect we're in multitest
        time: Timestamp::from_seconds(old_block_info.time.seconds() + 74056),
        chain_id: old_block_info.chain_id,
    });

    // and we can spend $2 now
    let can_spend_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        TERRA_MAINNET_AXLUSDC_IBC,
        2_000_000u128,
    )
    .unwrap();
    assert!(can_spend_response.can_execute == CanExecute::Yes(AllowanceWithReset));
    if let CanExecute::Yes(reason) = can_spend_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 3j: We can spend 2 Juno now as well");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily,
        "ujunox",
        2_000_000u128,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::Yes(AllowanceWithReset));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");
}

#[test]
fn message_restrictions() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title(
        "Test 4a: Non-owner cannot execute the Kobayashi Maru action, not even without funds",
    );
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "kirk",
        "cheat",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::No(NoMatchingRule));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();

    print_test_title(
        "Test 4b: Add authorization for bob to KobayashiMaru, with 'kirk' and 'cheat' ",
    );
    let add_authorization_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: Some(vec![o.contract_addresses.dummy_enterprise.to_string()]),
                message_name: Some("MsgExecuteContract".to_string()),
                // remember in direct cases, this should be snake_case
                wasmaction_name: Some("kobayashi_maru".to_string()),
                fields: Some(vec![
                    (
                        KeyValueOptions {
                            key: String::from("captain"),
                            allowed_values: vec![StringOrBinary {
                                string: Some(String::from("kirk")),
                                binary: None,
                            }],
                        },
                        Some(FieldComp::Equals),
                    ),
                    (
                        KeyValueOptions {
                            key: String::from("strategy"),
                            allowed_values: vec![StringOrBinary {
                                string: Some(String::from("cheat")),
                                binary: None,
                            }],
                        },
                        Some(FieldComp::Equals),
                    ),
                ]),
                expiration: 0,
            }),
        },
    };
    let contract_type = ContractType::UserAccount;

    execute_test_msg(
        o.legacy_owner.to_string(),
        add_authorization_msg,
        contract_type,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    // print out our authorizations
    // disabled for now after migration to unified state
    /*
    println!("Current authorizations:");
    let query_msg = classes::msg_gatekeeper_message::QueryMsg::Authorizations {
        identifier: None,
        actor: None,
        target_contract: Some(o.contract_addresses.dummy_enterprise.to_string()),
        message_name: None,
        wasmaction_name: None,
        fields: None,
        limit: None,
        start_after: None,
    };
    let _authorizations_response: Authorizations = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.message_gatekeeper.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    println!();
    */

    print_test_title(
        "Test 4c: Can the authorized actor execute Kobayashi Maru with the wrong fields?",
    );
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "picard",
        "engage",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::No(NoMatchingRule));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    println!("{}...of course not, it's impossible{}", GREEN, WHITE);
    println!();

    print_test_title("Test 4d: What about the right captain, but the wrong strategy?");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "kirk",
        "seduce",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::No(NoMatchingRule));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    println!(
        "{}...nope. One too many Priceline commercials.{}",
        GREEN, WHITE,
    );
    println!();

    print_test_title("Test 4e: But if both fields match the authorization...");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "kirk",
        "cheat",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::Yes(NoFundsAndAllowlist));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    println!(
        "{}...success. Unlike Jimmy T, Bob can't cheat.{}",
        GREEN, WHITE,
    );
    println!();

    println!("Making sure no cheaters have been detected yet...");
    let query_msg = dummy_counter_executable::msg::QueryMsg::CheaterDetected {};
    let cheater_detected_response: CheaterDetectedResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.dummy_enterprise.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    println!(
        "\tresponse: cheater_detected: {}",
        cheater_detected_response.cheater_detected,
    );
    assert!(!cheater_detected_response.cheater_detected);
    println!("{}...ok, no cheaters so far.{}", GREEN, WHITE);
    println!();

    print_test_title("Test 4f: Finally, let's make sure message is actually executed.");
    let execute_msg = dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
        captain: "kirk".to_string(),
        strategy: "cheat".to_string(),
    };
    let wrapped_execute_msg = classes::msg_user_entry::ExecuteMsg::Execute {
        msg: to_binary(&UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
            LegacyMsg::WasmMsg::Execute {
                contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                msg: to_binary(&execute_msg).unwrap(),
                funds: vec![],
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                code_hash: "dummy_hash".to_string(),
            },
        )))
        .unwrap(),
        signatures: None,
    };
    execute_test_msg(
        o.authorized_spender_daily.to_string(),
        wrapped_execute_msg,
        ContractType::UserEntry,
        &mut router,
        &o,
    )
    .unwrap();

    println!("Making sure cheaters have been detected...");
    let query_msg = dummy_counter_executable::msg::QueryMsg::CheaterDetected {};
    let cheater_detected_response: CheaterDetectedResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.dummy_enterprise.clone(),
                o.contract_addresses,
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    println!(
        "\tresponse: cheater_detected: {}",
        cheater_detected_response.cheater_detected,
    );
    assert!(cheater_detected_response.cheater_detected);
    print_success("");
}

#[test]
fn sessionkeys() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title(
        "Test 5a: Let's create a Session Key ... that will temporarily enable any message",
    );
    let add_sessionkey_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_sessionkey_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 5b: Now, we can send with any fields");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "janeway",
        "philosophize",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    println!("can_execute_response: {:?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("");

    print_success("But if we advance time...");
    let old_block_info = router.block_info();
    router.set_block(BlockInfo {
        height: 12345u64, // used sometimes to detect we're in multitest
        time: Timestamp::from_seconds(old_block_info.time.seconds() + 86400),
        chain_id: old_block_info.chain_id,
    });

    print_test_title("Test 5c: Now the same fails");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "janeway",
        "philosophize",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::No(CannotExecuteReason::RuleExpired));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 5d: Let's create that Session Key again");
    let add_allowlist_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_allowlist_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 5e: As before, we can now act");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "janeway",
        "philosophize",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 5f: The Session Key can destroy itself using the pass through");
    // get rule id
    let query_msg = classes::msg_user_state::QueryMsg::AbstractionRules {
        actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
        ty: vec![GatekeeperType::Allowlist],
    };
    let rules_response: AbstractionRules = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_state.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();

    let destroy_sessionkey_msg = classes::msg_user_state::ExecuteMsg::RmAbstractionRule {
        ty: GatekeeperType::Allowlist,
        rule_id: rules_response.rules[rules_response.rules.len() - 1]
            .id
            .unwrap(),
    };
    execute_test_msg(
        o.authorized_spender_daily.clone(),
        destroy_sessionkey_msg,
        ContractType::UserState,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 5g: And now cannot act");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "janeway",
        "philosophize",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::No(CannotExecuteReason::RuleExpired));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");
}

#[test]
fn sessionkey_sub_rules() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title(
        "Test 9a: Now, create a Session Key with a message restriction (can only act as Janeway).",
    );
    let add_sessionkey_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: Some(vec![o.contract_addresses.dummy_enterprise.to_string()]),
                message_name: None,
                wasmaction_name: None,
                fields: Some(vec![(
                    KeyValueOptions {
                        key: String::from("captain"),
                        allowed_values: vec![StringOrBinary {
                            string: Some(String::from("janeway")),
                            binary: None,
                        }],
                    },
                    Some(FieldComp::Equals),
                )]),
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_sessionkey_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 9b: Now we cannot act as kirk");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "kirk",
        "philosophize",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(
        can_execute_response.can_execute == CanExecute::No(CannotExecuteReason::NoMatchingRule)
    );
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 9c: But we can act as janeway (note we must specify at least one thing other than fields)");
    let can_execute_response = check_kobayashi(
        o.authorized_spender_daily.clone(),
        "janeway",
        "philosophize",
        o.contract_addresses.clone(),
        &router,
    )
    .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    print_success("");

    let query_msg = classes::msg_user_state::QueryMsg::AbstractionRules {
        actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
        ty: vec![GatekeeperType::Allowlist],
    };
    let rules_response: AbstractionRules = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_state.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();

    print_test_title("destroying session key again...");
    let destroy_sessionkey_msg = classes::msg_user_state::ExecuteMsg::RmAbstractionRule {
        ty: GatekeeperType::Allowlist,
        rule_id: rules_response.rules[rules_response.rules.len() - 1]
            .id
            .unwrap(),
    };
    execute_test_msg(
        o.authorized_spender_daily.clone(),
        destroy_sessionkey_msg,
        ContractType::UserState,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 9d: Let's make a Session Key with a spend limit restriction");
    // under refactored arch, this requires two rules:
    // an Allowlist rule with just the actor specified,
    // and a Spendlimit rule for the same actor
    let add_allowlist_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    let add_spendlimit_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_daily.clone(),
                cooldown: 0,                   // not relevant for session key sub_rules!
                period_type: PeriodType::Days, // not relevant for session key sub_rules!
                period_multiple: 1,            // not relevant for session key sub_rules!
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                    amount: Uint256::from(69420u128),
                    limit_remaining: Uint256::from(69420u128),
                    spent_this_inheritance_period: None,
                }],
                offset: 0u32,
                denom: Some(TERRA_MAINNET_AXLUSDC_IBC.to_string()),
                default: None,
                inheritance_records: vec![],
            }),
        },
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_allowlist_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_spendlimit_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 9e: Try (and fail) to spend 69421 uusdc");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        TERRA_MAINNET_AXLUSDC_IBC,
        69_421u128,
    )
    .unwrap();
    println!("final can_execute_response: {:#?}", can_execute_response);
    assert!(can_execute_response.can_execute == CanExecute::No(AllowanceExceeded));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();

    print_test_title("Test 9f: But spend 69420 successfully");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        TERRA_MAINNET_AXLUSDC_IBC,
        69_420u128,
    );
    let unwrapped_res = can_execute_response.unwrap();
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    assert!(matches!(unwrapped_res.can_execute, CanExecute::Yes(_)));
    print_success("");
}

#[test]
fn message_restriction_comparisons() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test 10a: This sessionkey has a strategy restriction: between 10 and 50");
    let add_sessionkey_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: Some(vec![o.contract_addresses.dummy_enterprise.to_string()]),
                message_name: None,
                wasmaction_name: None,
                fields: Some(vec![
                    (
                        KeyValueOptions {
                            key: String::from("strategy"),
                            allowed_values: vec![StringOrBinary {
                                string: Some("10".to_string()),
                                binary: None,
                            }],
                        },
                        Some(FieldComp::GreaterThanOrEqual),
                    ),
                    (
                        KeyValueOptions {
                            key: String::from("strategy"),
                            allowed_values: vec![StringOrBinary {
                                string: Some("50".to_string()),
                                binary: None,
                            }],
                        },
                        Some(FieldComp::LessThanOrEqual),
                    ),
                ]),
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_sessionkey_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 10b: Now we can't run with strategy of 9...");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "kirk".to_string(),
                            strategy: "9".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::No(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 10c: Nor 51...");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "kirk".to_string(),
                            strategy: "51".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::No(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 10d: But 50 is fine!");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "kirk".to_string(),
                            strategy: "50".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 10e: New key: Picard can either 'engage' or 'assimilate'");
    let add_sessionkey_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: Some(vec![o.contract_addresses.dummy_enterprise.to_string()]),
                message_name: None,
                wasmaction_name: None,
                fields: Some(vec![
                    (
                        KeyValueOptions {
                            key: String::from("captain"),
                            allowed_values: vec![StringOrBinary {
                                string: Some("picard".to_string()),
                                binary: None,
                            }],
                        },
                        Some(FieldComp::Equals),
                    ),
                    (
                        KeyValueOptions {
                            key: String::from("strategy"),
                            allowed_values: vec![
                                StringOrBinary {
                                    string: Some("engage".to_string()),
                                    binary: None,
                                },
                                StringOrBinary {
                                    string: Some("assimilate".to_string()),
                                    binary: None,
                                },
                            ],
                        },
                        Some(FieldComp::AnyOf),
                    ),
                ]),
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_sessionkey_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test 10f: Can't assimilate with Janeway...");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "janeway".to_string(),
                            strategy: "assimilate".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::No(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 10g: Can't cheat with Picard...");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "picard".to_string(),
                            strategy: "cheat".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::No(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 10h: But can assimilate with Picard!");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "picard".to_string(),
                            strategy: "assimilate".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 10i: Or engage with Picard");
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(&dummy_counter_executable::msg::ExecuteMsg::KobayashiMaru {
                            captain: "picard".to_string(),
                            strategy: "engage".to_string(),
                        })
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");
}

#[test]
fn message_restriction_deep_comparison() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title(
        "Test 12a: Set up deep comparison: Picard can either 'engage' or 'assimilate'",
    );
    let add_sessionkey_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: Some(vec![o.contract_addresses.dummy_enterprise.to_string()]),
                message_name: None,
                wasmaction_name: None,
                fields: Some(vec![
                    (
                        KeyValueOptions {
                            key: String::from("captain"),
                            allowed_values: vec![StringOrBinary {
                                string: Some("picard".to_string()),
                                binary: None,
                            }],
                        },
                        Some(FieldComp::Equals),
                    ),
                    (
                        KeyValueOptions {
                            key: String::from("strategies"),
                            allowed_values: vec![
                                StringOrBinary {
                                    string: None,
                                    binary: Some(
                                        to_binary(&Substrategy {
                                            strategy: "engage".to_string(),
                                            authorized: "<ANY>".to_string(),
                                        })
                                        .unwrap(),
                                    ),
                                },
                                StringOrBinary {
                                    string: None,
                                    binary: Some(
                                        to_binary(&Substrategy {
                                            strategy: "assimilate".to_string(),
                                            authorized: "<ANY>".to_string(),
                                        })
                                        .unwrap(),
                                    ),
                                },
                            ],
                        },
                        Some(FieldComp::AnyMatchingObject),
                    ),
                ]),
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    println!(
        "ALERT: add_sessionkey_msg: {:#?}",
        to_binary(&add_sessionkey_msg).unwrap(),
    );
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_sessionkey_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title(
        "Test 12b: Picard can deep assimilate (Run deep comparison with nested strategies)",
    );
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(
                            &dummy_counter_executable::msg::ExecuteMsg::DeepKobayashiMaru {
                                captain: "picard".to_string(),
                                strategies: vec![Substrategy {
                                    strategy: "assimilate".to_string(),
                                    authorized: "no".to_string(),
                                }],
                            },
                        )
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title(
        "Test 12c: Picard can deep engage (Run deep comparison with nested strategies)",
    );
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(
                            &dummy_counter_executable::msg::ExecuteMsg::DeepKobayashiMaru {
                                captain: "picard".to_string(),
                                strategies: vec![Substrategy {
                                    strategy: "engage".to_string(),
                                    authorized: "yes".to_string(),
                                }],
                            },
                        )
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::Yes(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title(
        "Test 12d: Picard CANNOT deep cheat (Run deep comparison with nested strategies)",
    );
    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            o.contract_addresses.user_account.to_string(),
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: o.authorized_spender_daily.clone(),
                msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(
                    LegacyMsg::WasmMsg::Execute {
                        contract_addr: o.contract_addresses.dummy_enterprise.to_string(),
                        msg: to_binary(
                            &dummy_counter_executable::msg::ExecuteMsg::DeepKobayashiMaru {
                                captain: "picard".to_string(),
                                strategies: vec![Substrategy {
                                    strategy: "cheat".to_string(),
                                    authorized: "hell no".to_string(),
                                }],
                            },
                        )
                        .unwrap(),
                        funds: vec![],
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: "dummy_hash".to_string(),
                    },
                )),
                funds: vec![],
            },
        )
        .unwrap();
    println!("can execute response: {:#?}", can_execute_response);
    assert!(matches!(
        can_execute_response.can_execute,
        CanExecute::No(_)
    ));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("");
}

#[test]
fn inheritance() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test group 6: For beneficiaries, let's add some balances.");
    mint_native(
        &mut router,
        o.legacy_owner.to_string(),
        "ujunox".to_string(),
        100_000u128,
    );
    router
        .send_tokens(
            o.legacy_owner.clone(),
            o.contract_addresses.user_account.clone(),
            &[Coin {
                denom: "ujunox".to_string(),
                amount: Uint128::from(100_000u128),
            }],
        )
        .unwrap();
    print_success("");

    print_test_title("Test 6a: Let's add a beneficiary. Full inheritance.");
    let execute_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.beneficiary_full.clone()),
            ty: GatekeeperType::Inheritance,
            main_rule: Rule::Inheritance(PermissionedAddressParams {
                address: o.beneficiary_full.clone(),
                cooldown: 1u64, // Inheritance active in 24hours. True degens would never be dormant that long.
                period_type: PeriodType::Days,
                period_multiple: 1,
                spend_limits: vec![], // no drip
                offset: 0u32,
                denom: None,
                default: Some(true),
                inheritance_records: vec![],
            }),
        },
    };
    let contract_addr = o.contract_addresses.user_account.clone();

    let _ = router
        .execute_contract(
            o.legacy_owner.clone(),
            use_contract(
                contract_addr,
                o.contract_addresses.clone(),
                "Execute".to_string(),
            ),
            &execute_msg,
            &[],
        )
        .unwrap();
    print_success("");

    print_test_title("Test 6b: Beneficiary can't spend anything yet");
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.beneficiary_full.clone(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Send {
            to_address: o.authorized_spender_daily.clone(),
            amount: vec![coin(100u128, "ujunox")],
        })),
    };
    let can_spend_response: Result<CanExecuteResponse, StdError> = router.wrap().query_wasm_smart(
        use_contract(
            o.contract_addresses.user_account.clone(),
            o.contract_addresses.clone(),
            "Query".to_string(),
        ),
        &query_msg,
    );
    let res: CanExecuteResponse = can_spend_response.unwrap();
    println!("\tresponse: can_spend: {:#?}", res);
    assert!(res.can_execute == CanExecute::No(BeneficiaryInheritanceNotActive));

    println!("{}...nope, degen dad is still kicking.{}", GREEN, WHITE);
    println!();

    print_test_title("Test 6c: Advance 1 day without other activity, tho...");
    let old_block_info = router.block_info();
    router.set_block(BlockInfo {
        height: 12345u64, // used sometimes to detect we're in multitest
        time: Timestamp::from_seconds(old_block_info.time.seconds() + 86400),
        chain_id: old_block_info.chain_id,
    });
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.beneficiary_full.clone(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Send {
            to_address: o.authorized_spender_daily.clone(),
            amount: vec![coin(100u128, "ujunox")],
        })),
    };
    let can_execute_response: Result<CanExecuteResponse, StdError> =
        router.wrap().query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        );
    let unwrapped_res = can_execute_response.unwrap();
    // Would reset since we just activated from dormancy,
    // but this is a full funds control permission
    println!("unwrapped_res: {:#?}", unwrapped_res);
    assert!(unwrapped_res.can_execute == CanExecute::Yes(BeneficiaryWithReset));
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    println!(
        "{}...degen dad must be dead. Inheritance unlocked.{}",
        GREEN, WHITE,
    );
    println!();
}

#[test]
fn monthly_spend_limits() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test 7a: Add a permissioned user with a $100 monthly spend limit");
    // Let's have bob added as a permissioned user
    #[cfg(feature = "backward_compat")]
    let msg = classes::gatekeeper_spendlimit::ExecuteMsg::UpsertPermissionedAddress {
        new_permissioned_address: PermissionedAddressParams {
            address: o.authorized_spender_monthly.clone(),
            cooldown: o.block_info.time.seconds().checked_add(2_628_000).unwrap(),
            period_type: PeriodType::Months,
            period_multiple: 1,
            offset: 0u32,
            spend_limits: vec![classes::permissioned_address::CoinBalance {
                denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                amount: 100_000_000u128,
                limit_remaining: 100_000_000u128,
                current_balance: 0u128,
            }],
            denom: Some("true".to_string()),
            default: Some(true),
            inheritance_records: vec![],
        },
    };
    #[cfg(not(feature = "backward_compat"))]
    let msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_monthly.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_monthly.clone(),
                cooldown: o.block_info.time.seconds().checked_add(2_628_000).unwrap(),
                period_type: PeriodType::Months,
                period_multiple: 1,
                offset: 0u32,
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                    amount: Uint256::from(100_000_000u128),
                    limit_remaining: Uint256::from(100_000_000u128),
                    spent_this_inheritance_period: None,
                }],
                denom: Some(TERRA_MAINNET_AXLUSDC_IBC.to_string()),
                default: Some(true),
                inheritance_records: vec![],
            }),
        },
    };
    #[cfg(feature = "backward_compat")]
    let contract_addr = o.contract_addresses.spendlimit_gatekeeper.clone();
    #[cfg(not(feature = "backward_compat"))]
    let contract_addr = o.contract_addresses.user_account.clone();

    let _ = router
        .execute_contract(
            o.legacy_owner,
            use_contract(
                contract_addr,
                o.contract_addresses.clone(),
                "Execute".to_string(),
            ),
            &msg,
            &[],
        )
        .unwrap();
    print_success("");

    // we have a $100 USDC spend limit, so we should be able to spend $99...
    // we could query with classes::gatekeeper_spendlimit::QueryMsg::CanSpend,
    // but this is an integration test
    print_test_title(
        "Test 7b: Check that permissioned user can spend $99, with auto $0.0001 repayment",
    );
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_monthly,
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Send {
            to_address: "bob".to_string(),
            amount: vec![Coin {
                denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                amount: Uint128::from(99_000_000u128),
            }],
        })),
    };

    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses,
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::Yes(Allowance));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("");

    // TODO: monthly resets!
}

#[test]
fn cw20_spend_limits() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants =
        obi_init_tests(&mut router, false, TERRA_MAINNET_AXLUSDC_IBC.to_string());

    print_test_title("Test 8a: Confirm we can some cw20 tokens as owner");

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.legacy_owner.to_string(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
            contract_addr: o.contract_addresses.dummy_cw20.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: o.authorized_spender_monthly.to_string(),
                amount: Uint128::from(99u128), // owner should have 420; no actual balance checking as can_execute doesn't simulate msg
            })
            .unwrap(),
            funds: vec![],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: "dummy_hash".to_string(),
        })),
    };

    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::Yes(OwnerNoDelay));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 8b: Random user cannot spend cw20");

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_daily.to_string(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
            contract_addr: o.contract_addresses.dummy_cw20.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: o.authorized_spender_monthly.to_string(),
                amount: Uint128::from(99u128), // owner should have 420; no actual balance checking as can_execute doesn't simulate msg
            })
            .unwrap(),
            funds: vec![],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: "dummy_hash".to_string(),
        })),
    };

    let can_execute_response: Result<CanExecuteResponse, StdError> =
        router.wrap().query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        );
    let unwrapped_res = can_execute_response.unwrap();
    println!("unwrapped_res: {:#?}", unwrapped_res);
    // should be
    // assert!(unwrapped_res.can_execute == CanExecute::No(AllowanceExceeded));
    assert!(matches!(unwrapped_res.can_execute, CanExecute::No(_)));
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();

    print_test_title("Test 8c: Ensure a direct query is returning false");

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_daily.clone(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
            contract_addr: o.contract_addresses.dummy_cw20.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: o.authorized_spender_monthly.clone(),
                amount: Uint128::from(99u128),
            })
            .unwrap(),
            funds: vec![],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: "dummy_hash".to_string(),
        })),
    };

    let can_spend_response: Result<CanSpendResponse, StdError> = router.wrap().query_wasm_smart(
        use_contract(
            o.contract_addresses.user_account.clone(),
            o.contract_addresses.clone(),
            "Query".to_string(),
        ),
        &query_msg,
    );
    let error = can_spend_response.unwrap_err();
    println!("\tresponse: can_spend: {}", error);
    print_success("...failed as expected");
    println!();

    print_success("Add a permissioned user with a $100 daily spend limit");
    // Let's have bob added as a permissioned user
    #[cfg(feature = "backward_compat")]
    let msg: SpendlimitExecuteMsg = SpendlimitExecuteMsg::UpsertPermissionedAddress {
        new_permissioned_address: PermissionedAddressParams {
            address: o.authorized_spender_daily.clone(),
            cooldown: o.block_info.time.seconds().checked_add(86400).unwrap(),
            period_type: PeriodType::Days,
            period_multiple: 1,
            offset: 0u32,
            spend_limits: vec![classes::permissioned_address::CoinBalance {
                denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                amount: 100_000_000u128,
                limit_remaining: 100_000_000u128,
                current_balance: 0u128,
            }],
            denom: Some("true".to_string()),
            default: Some(true),
            inheritance_records: vec![],
        },
    };
    #[cfg(not(feature = "backward_compat"))]
    let msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_daily.clone(),
                cooldown: o.block_info.time.seconds().checked_add(86400).unwrap(),
                period_type: PeriodType::Days,
                period_multiple: 1,
                offset: 0u32,
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
                    amount: Uint256::from(100_000_000u128),
                    limit_remaining: Uint256::from(100_000_000u128),
                    spent_this_inheritance_period: None,
                }],
                denom: Some(TERRA_MAINNET_AXLUSDC_IBC.to_string()),
                default: Some(true),
                inheritance_records: vec![],
            }),
        },
    };
    #[cfg(feature = "backward_compat")]
    let contract_addr = o.contract_addresses.spendlimit_gatekeeper.clone();
    #[cfg(not(feature = "backward_compat"))]
    let contract_addr = o.contract_addresses.user_account.clone();

    let res: Result<AppResponse, anyhow::Error> = router.execute_contract(
        o.legacy_owner.clone(),
        use_contract(
            contract_addr,
            o.contract_addresses.clone(),
            "Execute".to_string(),
        ),
        &msg,
        &[],
    );
    res.unwrap();
    print_success("");

    print_test_title("Test 8d: Now authorized spender can spend cw20 tokens");

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_daily.to_string(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
            contract_addr: o.contract_addresses.dummy_cw20.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: o.authorized_spender_monthly.to_string(),
                amount: Uint128::from(99u128), // owner should have 420; no actual balance checking as can_execute doesn't simulate msg
            })
            .unwrap(),
            funds: vec![],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: "dummy_hash".to_string(),
        })),
    };

    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses.clone(),
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    assert!(can_execute_response.can_execute == CanExecute::Yes(Allowance));
    if let CanExecute::Yes(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("");

    print_test_title("Test 8e: But not over spend limit");

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_daily.to_string(),
        funds: vec![],
        msg: UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
            contract_addr: o.contract_addresses.dummy_cw20.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: o.authorized_spender_monthly.to_string(),
                amount: Uint128::from(999_999_999_999_999_999u128), // owner should have 420; no actual balance checking as can_execute doesn't simulate msg
            })
            .unwrap(),
            funds: vec![],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: "dummy_hash".to_string(),
        })),
    };

    let can_execute_response: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(
            use_contract(
                o.contract_addresses.user_account.clone(),
                o.contract_addresses,
                "Query".to_string(),
            ),
            &query_msg,
        )
        .unwrap();
    println!("binary: {}", to_binary(&can_execute_response).unwrap());
    assert!(can_execute_response.can_execute == CanExecute::No(AllowanceExceeded));
    if let CanExecute::No(reason) = can_execute_response.can_execute {
        println!("\tresponse: can_spend: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();
}

#[test]
#[cfg(feature = "cosmwasm")]
fn osmosis_tests() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants = obi_init_tests(&mut router, false, "uosmo".to_string());

    let osmo_msg = UniversalMsg::Osmo(classes::universal_msg::OsmoMsg::SwapExactAmountIn(
        MsgSwapExactAmountIn {
            sender: o.contract_addresses.user_account.to_string(),
            routes: vec![SwapAmountInRoute {
                pool_id: 12,
                token_out_denom:
                    "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477"
                        .to_string(),
            }],
            token_in: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                denom: "uosmo".to_string(),
                amount: "1000000".to_string(),
            }),
            token_out_min_amount: "1000".to_string(),
        },
    ));

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.legacy_owner.to_string(),
        msg: osmo_msg,
        funds: vec![],
    };

    let res: classes::user_account::CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(o.contract_addresses.user_account.clone(), &query_msg)
        .unwrap();
    assert!(matches!(res.can_execute, CanExecute::Yes(_)));

    // basic test to join a pool
    let osmo_msg = UniversalMsg::Osmo(classes::universal_msg::OsmoMsg::JoinPool(MsgJoinPool {
        sender: o.contract_addresses.user_account.to_string(),
        pool_id: 12,
        share_out_amount: "1".to_string(),
        token_in_maxs: vec![osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: "uosmo".to_string(),
            amount: "1000".to_string(),
        }],
    }));

    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.legacy_owner.to_string(),
        msg: osmo_msg,
        funds: vec![],
    };

    let res: classes::user_account::CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(o.contract_addresses.user_account.clone(), &query_msg)
        .unwrap();
    assert!(matches!(res.can_execute, CanExecute::Yes(_)));

    // create a session key with spendlimit in uosmo
    // this is a double rule in new architecture (expiring message allow + spendlimit)
    let add_allowlist_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    let add_spendlimit_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_daily.clone(),
                cooldown: 0,                   // not relevant for session key sub_rules!
                period_type: PeriodType::Days, // not relevant for session key sub_rules!
                period_multiple: 1,            // not relevant for session key sub_rules!
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: "uosmo".to_string(),
                    amount: Uint256::from(5_000_000u128),
                    limit_remaining: Uint256::from(5_000_000u128),
                    spent_this_inheritance_period: None,
                }],
                offset: 0u32,
                denom: Some("uosmo".to_string()),
                default: None,
                inheritance_records: vec![],
            }),
        },
    };
    println!(
        "OSMOSIS SPENDLIMIT MSG: {:#?}",
        to_binary(&add_allowlist_msg).unwrap(),
    );
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_allowlist_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    execute_test_msg(
        o.legacy_owner.to_string(),
        add_spendlimit_msg,
        ContractType::UserAccount,
        &mut router,
        &o,
    )
    .unwrap();
    print_success("");

    print_test_title("Test osmo spend fails...");
    let can_spend_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        "uosmo",
        5_000_001u128,
    )
    .unwrap();
    assert!(can_spend_response.can_execute == CanExecute::No(AllowanceExceeded));
    if let CanExecute::No(reason) = can_spend_response.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    print_success("...failed as expected");
    println!();

    print_test_title("Test osmo spend succeeds...");
    let can_execute_response = check_spend(
        &router,
        o.contract_addresses.clone(),
        o.authorized_spender_daily.clone(),
        "uosmo",
        5_000_000u128,
    );
    let unwrapped_res = can_execute_response.unwrap();
    if let CanExecute::Yes(reason) = unwrapped_res.can_execute {
        println!("\tresponse: can_execute: {}", readable(reason as u8));
    }
    assert!(matches!(unwrapped_res.can_execute, CanExecute::Yes(_)));
    print_success("");
}

#[test]
fn serde_bin() {
    let mut router = mock_app();
    let o: ObiTestConstants = obi_init_tests(&mut router, false, "uosmo".to_string());

    let _msg: Binary = Binary::from_base64("eyJhZGRfYWJzdHJhY3Rpb25fcnVsZSI6eyJuZXdfcnVsZSI6eyJhY3RvciI6Im9zbW8xbjdkM3R4bDdqa3BjY3VzZzRoeHg4c3Z0Z2c4d256MzcyazVscmoiLCJ0eSI6InNlc3Npb25rZXkiLCJtYWluX3J1bGUiOnsic2Vzc2lvbl9rZXkiOnsiZXhwaXJhdGlvbiI6MTY4NTU2NTkyMSwiYWRtaW5fcGVybWlzc2lvbnMiOmZhbHNlfSwic3ViX3J1bGVzIjpbWyJhbGxvd2xpc3QiLHsiYWxsb3ciOnsiZmllbGRzIjpbeyJrZXkiOiJyb3V0ZXMiLCJhbGxvd2VkX3ZhbHVlcyI6W3siYmluYXJ5IjoiZXlKd2IyOXNYMmxrSWpveE1Dd2lkRzlyWlc1ZmIzVjBYMlJsYm05dElqb2lhV0pqTHpKRk56TTJPRUV4TkVGRE9VRkNOemczTUVZek1rTkdSVUUyT0RjMU5URkROVEEyTkVaQk9EWXhPRFk0UlVSR056UXpOMEpET0RjM016VTRRVGd4UmpraWZRPT0ifV19LCJhbnlfbWF0Y2hpbmdfb2JqZWN0Il19fV1dfX19fQ==").unwrap();

    let add_sessionkey_msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked(o.authorized_spender_daily.clone())),
                // note a fields-only sub_rule will fail. At least one other item must match
                contract: Some(vec![o.contract_addresses.dummy_enterprise.to_string()]),
                message_name: None,
                wasmaction_name: None,
                fields: Some(vec![
                    (KeyValueOptions {
                        key: String::from("routes"),
                        allowed_values: vec![
                            StringOrBinary {
                                string: None,
                                binary: Some(to_binary(&SwapAmountInRoute {
                                    pool_id: 10,
                                    token_out_denom: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9".to_string(),
                                }).unwrap())
                            },
                            StringOrBinary {
                                string: None,
                                binary: Some(to_binary(&SwapAmountInRoute {
                                    pool_id: 12,
                                    token_out_denom: "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477".to_string(),
                                }).unwrap()),
                            }
                        ]},
                        Some(FieldComp::AnyMatchingObject),
                    ),
                ]),
                expiration: router.block_info().time.plus_seconds(60).seconds(),
            }),
        },
    };
    println!(
        "add_sessionkey_msg: {:#?}",
        to_binary(&add_sessionkey_msg).unwrap(),
    );
}

#[test]
fn eth_user_op_tests() {
    // Set up mock multi-test app
    let mut router = mock_app();

    // Set up everything else
    let o: ObiTestConstants = obi_init_tests(
        &mut router,
        false,
        "5cf29823ccfc73008fa53630d54a424ab82de6f2".to_string(),
    );

    // Add a spendlimit user
    // with a spendlimit of 0.001 (in 18 ether digits)
    let msg = classes::msg_user_account::ExecuteMsg::AddAbstractionRule {
        new_rule: AbstractionRule {
            id: None,
            actor: Addr::unchecked(o.authorized_spender_daily.clone()),
            ty: GatekeeperType::Spendlimit,
            main_rule: Rule::Spendlimit(PermissionedAddressParams {
                address: o.authorized_spender_daily.clone(),
                cooldown: o.block_info.time.seconds().checked_add(86400).unwrap(),
                period_type: PeriodType::Days,
                period_multiple: 1,
                offset: 0u32,
                spend_limits: vec![classes::permissioned_address::CoinBalance {
                    denom: "5cf29823ccfc73008fa53630d54a424ab82de6f2".to_string(),
                    amount: Uint256::from(1_000_000_000_000_000u128),
                    limit_remaining: Uint256::from(1_000_000_000_000_000u128),
                    spent_this_inheritance_period: None,
                }],
                denom: Some("5cf29823ccfc73008fa53630d54a424ab82de6f2".to_string()),
                default: Some(true),
                inheritance_records: vec![],
            }),
        },
    };

    let _res = router
        .execute(
            o.legacy_owner.clone(),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: o.contract_addresses.user_account.to_string(),
                msg: to_binary(&msg).unwrap(),
                funds: vec![],
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                code_hash: "dummy_hash".to_string(),
            }),
        )
        .unwrap();

    // see if can_execute a spend of 38d7ea4c6801... fails
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_daily.clone(),
        msg: UniversalMsg::Eth(EthUserOp {
            sender: o.authorized_spender_daily.clone(),
            nonce: Uint256::from(1u64),
            init_code: vec![],
            // spending 1 with 18 decimals, + 1*10-18
            call_data: hex::decode("b61d27f60000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d6000000000000000000000000000000000000000000000000000038d7ea4c6801000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: Uint256::from(79118u64),
            verification_gas_limit: Uint256::from(146778u64),
            pre_verification_gas: Uint256::from(48172u64),
            max_fee_per_gas: Uint256::from(10327u64),
            max_priority_fee_per_gas: Uint256::from(429u64),
            paymaster_and_data: hex::decode("e93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064b564460000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003495a86706e134c9c162d4a020097db67f95673294f1d7a42608633046b13ab7130f5918760717b4a74c347c595d38fc2bdeaa7dc17429a9f6c1ac3f344266e91b").unwrap(),
            signature: vec![]
        }),
        funds: vec![],
    };

    let res: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(o.contract_addresses.user_account.clone(), &query_msg)
        .unwrap();

    assert!(matches!(res.can_execute, CanExecute::No(_)));

    // but the 0.001 even should succeed
    let query_msg = classes::msg_user_account::QueryMsg::CanExecute {
        address: o.authorized_spender_daily.clone(),
        msg: UniversalMsg::Eth(EthUserOp {
            sender: o.authorized_spender_daily.clone(),
            nonce: Uint256::from(1u64),
            init_code: vec![],
            // spending 0.001 with 18 decimals
            call_data: hex::decode("b61d27f60000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d6000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: Uint256::from(79118u64),
            verification_gas_limit: Uint256::from(146778u64),
            pre_verification_gas: Uint256::from(48172u64),
            max_fee_per_gas: Uint256::from(10327u64),
            max_priority_fee_per_gas: Uint256::from(429u64),
            paymaster_and_data: hex::decode("e93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064b564460000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003495a86706e134c9c162d4a020097db67f95673294f1d7a42608633046b13ab7130f5918760717b4a74c347c595d38fc2bdeaa7dc17429a9f6c1ac3f344266e91b").unwrap(),
            signature: vec![]
        }),
        funds: vec![],
    };

    let res: CanExecuteResponse = router
        .wrap()
        .query_wasm_smart(o.contract_addresses.user_account.to_string(), &query_msg)
        .unwrap();

    assert!(matches!(res.can_execute, CanExecute::Yes(_)));
}
