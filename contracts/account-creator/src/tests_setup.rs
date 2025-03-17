classes::cosmwasm_imports!(coin, Addr, Empty, Uint128, BlockInfo);
use classes::{
    account_creator::Config, msg_user_entry::UserAccountAddressResponse,
    pair_contract::SwapRouteResponse, user_account::GatekeeperContractsResponse,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use dummy_counter_executable::msg::InstantiateMsg;

use crate::{
    colors::{BLUE, FORCED_WHITE, GREEN, WHITE, YELLOW_UNDERLINE},
    tests_helpers::factory_deploy,
    tests_pair_registry::process_pair_contracts_json,
};

#[derive(Clone, Debug)]
pub struct ObiTestConstants {
    pub legacy_owner: Addr,
    pub new_owner: Addr,
    pub obi_owner: Addr,

    pub code_ids: CodeIds,
    pub authorized_spender_daily: String,
    pub authorized_spender_monthly: String,
    pub beneficiary_full: String,
    pub beneficiary_drip: String,
    pub block_info: BlockInfo,

    pub contract_addresses: ContractAddresses,
}

#[derive(Clone, Debug)]
pub struct ContractAddresses {
    pub account_creator: Option<Addr>,
    pub asset_unifier: Addr,
    pub spendlimit_gatekeeper: Addr,
    pub message_gatekeeper: Addr,
    pub user_account: Addr,
    pub dummy_cw20: Addr,
    pub dummy_enterprise: Addr,
    pub pair_registry: Addr,
    pub user_state: Addr,
    pub user_entry: Addr,
}

impl Default for ContractAddresses {
    fn default() -> Self {
        Self {
            account_creator: None,
            asset_unifier: Addr::unchecked("uninitialized"),
            spendlimit_gatekeeper: Addr::unchecked("uninitialized"),
            message_gatekeeper: Addr::unchecked("uninitialized"),
            user_account: Addr::unchecked("uninitialized"),
            dummy_cw20: Addr::unchecked("uninitialized"),
            dummy_enterprise: Addr::unchecked("uninitialized"),
            pair_registry: Addr::unchecked("uninitialized"),
            user_state: Addr::unchecked("uninitialized"),
            user_entry: Addr::unchecked("uninitialized"),
        }
    }
}

impl ContractAddresses {
    pub fn read_gatekeepers(&mut self, router: &App) {
        println!("User account contract is {}", self.user_account);
        let gatekeepers: GatekeeperContractsResponse = router
            .wrap()
            .query_wasm_smart(
                self.user_account.clone(),
                &classes::msg_user_account::QueryMsg::GatekeeperContracts {},
            )
            .unwrap();
        self.message_gatekeeper = Addr::unchecked(&gatekeepers.gatekeepers[0]);
        self.spendlimit_gatekeeper = Addr::unchecked(&gatekeepers.gatekeepers[1]);
        self.user_state = Addr::unchecked(gatekeepers.user_state_contract_addr.unwrap());
    }
}

#[allow(dead_code)]
pub fn mock_app() -> App {
    App::default()
}

#[allow(dead_code)]
pub fn account_creator_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

#[allow(dead_code)]
pub fn asset_unifier_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        asset_unifier::contract::execute,
        asset_unifier::contract::instantiate,
        asset_unifier::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn dummy_cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn dummy_dex_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dummy_price_contract::contract::execute,
        dummy_price_contract::contract::instantiate,
        dummy_price_contract::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn dummy_executable_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dummy_counter_executable::contract::execute,
        dummy_counter_executable::contract::instantiate,
        dummy_counter_executable::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn gatekeeper_spendlimit_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        gatekeeper_spendlimit::contract::execute,
        gatekeeper_spendlimit::contract::instantiate,
        gatekeeper_spendlimit::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn gatekeeper_message_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        gatekeeper_message::contract::execute,
        gatekeeper_message::contract::instantiate,
        gatekeeper_message::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn debtkeeper_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        debtkeeper::contract::execute,
        debtkeeper::contract::instantiate,
        debtkeeper::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn pair_registry_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        pair_registry::contract::execute,
        pair_registry::contract::instantiate,
        pair_registry::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn user_account_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        user_account::contract::execute,
        user_account::contract::instantiate,
        user_account::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn user_state_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        user_state::contract::execute,
        user_state::contract::instantiate,
        user_state::contract::query,
    );
    Box::new(contract)
}

#[allow(dead_code)]
pub fn user_entry_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        user_entry::contract::execute,
        user_entry::contract::instantiate,
        user_entry::contract::query,
    )
    .with_reply(user_entry::contract::reply);
    Box::new(contract)
}

pub fn asset_unifier_instantiate_msg(
    obi_owner: Option<String>,
    pair_contract_registry: String,
    default_asset: String,
) -> classes::asset_unifier::InstantiateMsg {
    classes::asset_unifier::InstantiateMsg {
        default_asset_unifier: default_asset,
        home_network: "multitest".to_string(),
        legacy_owner: obi_owner,
        pair_contract_registry,
    }
}

#[derive(Clone, Debug)]
pub struct CodeIds {
    pub account_creator: u64,
    pub asset_unifier: u64,
    pub dummy_cw20: u64,
    pub dummy_dex: u64,
    pub dummy_enterprise: u64,
    pub debtkeeper: u64,
    pub gatekeeper_spendlimit: u64,
    pub gatekeeper_spendlimit_pre_migrate: u64,
    pub gatekeeper_message: u64,
    pub pair_registry: u64,
    pub user_account: u64,
    pub user_account_pre_migrate: u64,
    pub user_state: u64,
    pub user_entry: u64,
}

pub fn get_code_ids(app: &mut App) -> CodeIds {
    CodeIds {
        account_creator: app.store_code(account_creator_contract()),
        asset_unifier: app.store_code(asset_unifier_contract()),
        dummy_cw20: app.store_code(dummy_cw20_contract()),
        dummy_dex: app.store_code(dummy_dex_contract()),
        dummy_enterprise: app.store_code(dummy_executable_contract()),
        debtkeeper: app.store_code(debtkeeper_contract()),
        gatekeeper_spendlimit: app.store_code(gatekeeper_spendlimit_contract()),
        gatekeeper_message: app.store_code(gatekeeper_message_contract()),
        pair_registry: app.store_code(pair_registry_contract()),
        user_account: app.store_code(user_account_contract()),
        // storing old code here as first test (in this file) is migrate to latest
        user_account_pre_migrate: app.store_code(user_account_contract()),
        gatekeeper_spendlimit_pre_migrate: app.store_code(gatekeeper_spendlimit_contract()),
        user_state: app.store_code(user_state_contract()),
        user_entry: app.store_code(user_entry_contract()),
    }
}

pub fn instantiate_contracts(
    router: &mut App,
    code_ids: CodeIds,
    obi_owner: Addr,
    _pre_migrate: bool, // true uses old user_account code for migration testing
    show_output: bool,
    default_asset: String,
) -> ContractAddresses {
    // kludge for now since secret doesn't support native migration
    let pre_migrate = false;
    // set up a cw20
    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "credits".to_string(),
        symbol: "CRED".to_string(),
        decimals: 6u8,
        initial_balances: vec![
            cw20::Cw20Coin {
                address: "owner".to_string(),
                amount: 420u128.into(),
            },
            cw20::Cw20Coin {
                address: "alice".to_string(),
                amount: 314u128.into(),
            },
        ],
        mint: None,
        marketing: None,
    };
    let cw20_contract_addr = router
        .instantiate_contract(
            code_ids.dummy_cw20,
            obi_owner.clone(),
            &cw20_instantiate,
            &[],
            "dummy_cw20",
            None,
        )
        .unwrap();

    // Add juno-test and terra-hermes entries to pair registry. Override "uloop" with contract above
    let (mocked_dummy_dex_contract_addrs, pair_registry_contract_addr) = setup_pairs_and_registry(
        router,
        code_ids.clone(),
        obi_owner.clone(),
        cw20_contract_addr.to_string(),
        show_output,
    );

    // Ensure dummy cw20 has an LP contract
    let cw20_pair_contract_addr: SwapRouteResponse = router
        .wrap()
        .query_wasm_smart(
            pair_registry_contract_addr.clone(),
            &classes::pair_registry::QueryMsg::SwapRoute {
                token0: "ujunox".to_string(),
                token1: cw20_contract_addr.to_string(),
            },
        )
        .unwrap();
    assert!(!cw20_pair_contract_addr.pair_routes.is_empty());

    // Instantiate the dummy *execute* contract using its stored code id
    let mocked_dummy_enterprise_contract_addr = router
        .instantiate_contract(
            code_ids.dummy_enterprise,
            obi_owner.clone(),
            &InstantiateMsg {},
            &[],
            "dummy_enterprise",
            None,
        )
        .unwrap();
    if show_output {
        println!(
            "Instantiating dummy enterprise contract. Code_id: {}{}{} Address: {}{}{}",
            BLUE,
            code_ids.dummy_enterprise,
            WHITE,
            BLUE,
            mocked_dummy_enterprise_contract_addr,
            WHITE,
        );
    }

    // Setup asset unifier price contract, using dummy price contract address
    let init_msg = asset_unifier_instantiate_msg(
        Some(obi_owner.to_string()),
        pair_registry_contract_addr.to_string(),
        default_asset,
    );
    // Instantiate the asset unifier contract
    let asset_unifier_contract_addr = router
        .instantiate_contract(
            code_ids.asset_unifier,
            obi_owner.clone(),
            &init_msg,
            &[],
            "asset_unifier",
            None,
        )
        .unwrap();
    if show_output {
        println!(
            "Instantiating asset unifier contract. Code_id: {}{}{} Address: {}{}{}",
            BLUE, code_ids.asset_unifier, WHITE, BLUE, asset_unifier_contract_addr, WHITE,
        );
    }

    let user_account_code_to_use = if pre_migrate {
        code_ids.user_account_pre_migrate
    } else {
        code_ids.user_account
    };

    let universal_gatekeeper_message_addr = router
        .instantiate_contract(
            code_ids.gatekeeper_message,
            obi_owner.clone(),
            &classes::gatekeeper_common::InstantiateMsg {},
            &[],
            "gatekeeper_message",
            None,
        )
        .unwrap();

    let universal_gatekeeper_spendlimit_addr = router
        .instantiate_contract(
            code_ids.gatekeeper_spendlimit,
            obi_owner.clone(),
            &classes::gatekeeper_spendlimit::InstantiateMsg {
                asset_unifier_contract: asset_unifier_contract_addr.to_string(),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                asset_unifier_code_hash: "dummy_hash".to_string(),
            },
            &[],
            "gatekeeper_spendlimit",
            None,
        )
        .unwrap();

    // Note the spendlimit is also premigrate, to test gatekeeper automigration
    let account_creator_msg = classes::account_creator::InstantiateMsg {
        owner: obi_owner.to_string(),
        config: Config {
            asset_unifier_address: asset_unifier_contract_addr.to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            asset_unifier_code_hash: "dummy_hash".to_string(),
            debt_repay_address: "repay_wallet".to_string(),
            debtkeeper_code_id: code_ids.debtkeeper,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            debtkeeper_code_hash: "dummy_hash".to_string(),
            #[cfg(feature = "cosmwasm")]
            default_gatekeepers: vec![
                (
                    code_ids.gatekeeper_message,
                    universal_gatekeeper_message_addr.to_string(),
                ),
                (
                    code_ids.gatekeeper_spendlimit,
                    universal_gatekeeper_spendlimit_addr.to_string(),
                ),
            ],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            default_gatekeepers: vec![
                (
                    code_ids.gatekeeper_message,
                    universal_gatekeeper_message_addr.to_string(),
                    "dummy_hash".to_string(),
                ),
                (
                    code_ids.gatekeeper_spendlimit,
                    universal_gatekeeper_spendlimit_addr.to_string(),
                    "dummy_hash".to_string(),
                ),
            ],
            user_account_code_id: user_account_code_to_use,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "dummy_hash".to_string(),
            fee_pay_address: "repay_wallet".to_string(),
            user_state_code_id: code_ids.user_state,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_state_code_hash: "dummy_hash".to_string(),
            user_entry_code_id: code_ids.user_entry,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_entry_code_hash: "dummy_hash".to_string(),
        },
    };

    // Instantiate the account creator contract
    let account_creator_addr = router
        .instantiate_contract(
            code_ids.account_creator,
            obi_owner,
            &account_creator_msg,
            &[],
            "account_creator",
            None,
        )
        .unwrap();
    if show_output {
        println!(
            "Instantiating account creator contract. Code_id: {}{}{} Address: {}{}{}",
            BLUE, code_ids.account_creator, WHITE, BLUE, account_creator_addr, WHITE,
        );
    }

    ContractAddresses {
        account_creator: Some(account_creator_addr),
        asset_unifier: asset_unifier_contract_addr,
        spendlimit_gatekeeper: Addr::unchecked("Undeployed"),
        message_gatekeeper: Addr::unchecked("Undeployed"),
        user_account: Addr::unchecked("Undeployed"),
        dummy_cw20: cw20_contract_addr,
        dummy_enterprise: mocked_dummy_enterprise_contract_addr,
        pair_registry: pair_registry_contract_addr,
        user_state: Addr::unchecked("Undeployed"),
        user_entry: Addr::unchecked("Undeployed"),
    }
}

pub fn use_contract(addy: Addr, contracts: ContractAddresses, ty: String) -> Addr {
    let contract_human_name = match addy.to_string() {
        val if val == contracts.account_creator.unwrap() => "Account Creator".to_string(),
        val if val == contracts.asset_unifier => "Asset Unifier".to_string(),
        val if val == contracts.pair_registry => "Pair Registry".to_string(),
        val if val == contracts.spendlimit_gatekeeper => "Spendlimit Gatekeeper".to_string(),
        val if val == contracts.message_gatekeeper => "Message Gatekeeper".to_string(),
        val if val == contracts.user_account => "User Account".to_string(),
        val if val == contracts.user_entry => "User Entry".to_string(),
        val if val == contracts.user_state => "User State".to_string(),
        val if val == contracts.dummy_enterprise => "U.S.S. Executable".to_string(),
        _ => "Unknown contract".to_string(),
    };
    match ty {
        val if val == *"Execute" => {
            println!("Calling contract: {}{}{}", BLUE, contract_human_name, WHITE);
        }
        val if val == *"Query" => {
            println!(
                "Querying contract: {}{}{}",
                BLUE, contract_human_name, WHITE,
            );
        }
        _ => panic!("bad type, use execute or query"),
    }
    addy
}

/// thanks William. grabbed from https://www.noveltech.dev/cw_multi_test_coin
pub fn mint_native(app: &mut App, recipient: String, denom: String, amount: u128) {
    app.sudo(cw_multi_test::SudoMsg::Bank(
        cw_multi_test::BankSudo::Mint {
            to_address: recipient,
            amount: vec![coin(amount, denom)],
        },
    ))
    .unwrap();
}

#[allow(clippy::type_complexity)]
pub fn setup_pairs_and_registry(
    router: &mut App,
    code_ids: CodeIds,
    obi_owner: Addr,
    override_test_cw20_address: String,
    show_output: bool,
) -> (Vec<((String, String), Addr)>, Addr) {
    // use juno pairs for local dummies
    let test_pairs = process_pair_contracts_json(
        "juno-test.json".to_string(),
        router,
        code_ids.dummy_dex,
        true,
        "dexadmin".to_string(),
    );

    // Setup dummy price contracts
    let mut mocked_dummy_dex_contract_addrs: Vec<((String, String), Addr)> = vec![];
    for test_pair in &test_pairs {
        let init_msg = dummy_price_contract::msg::InstantiateMsg {
            token0: if test_pair.token0 == *"uloop" {
                override_test_cw20_address.clone()
            } else {
                test_pair.token0.clone()
            },
            token1: if test_pair.token1 == *"uloop" {
                override_test_cw20_address.clone()
            } else {
                test_pair.token1.clone()
            },
            price: Uint128::from(test_pair.dummyprice.unwrap()),
        };

        // Instantiate the dummy price contract using its stored code id
        let mocked_dummy_dex_contract_addr = router
            .instantiate_contract(
                code_ids.dummy_dex,
                obi_owner.clone(),
                &init_msg,
                &[],
                "dummy_price",
                None,
            )
            .unwrap();
        if show_output {
            println!(
                "Instantiating dummy dex contract for {} to {}. Code_id: {}{}{} Address: {}{}{}",
                test_pair.token0,
                test_pair.token1,
                BLUE,
                code_ids.dummy_dex,
                WHITE,
                BLUE,
                mocked_dummy_dex_contract_addr,
                WHITE,
            );
        }
        mocked_dummy_dex_contract_addrs.push((
            (test_pair.token0.clone(), test_pair.token1.clone()),
            mocked_dummy_dex_contract_addr,
        ));
    }

    // Setup pair registry contract
    let init_msg = classes::pair_registry::InstantiateMsg {
        legacy_owner: Some(obi_owner.to_string()),
    };
    // Instantiate the pair registry contract
    let pair_registry_contract_addr = router
        .instantiate_contract(
            code_ids.pair_registry,
            obi_owner.clone(),
            &init_msg,
            &[],
            "pair_registry",
            None,
        )
        .unwrap();
    if show_output {
        println!(
            "Instantiating pair registry contract. Code_id: {}{}{} Address: {}{}{}",
            BLUE, code_ids.pair_registry, WHITE, BLUE, pair_registry_contract_addr, WHITE,
        );
    }

    // add juno pairs (actually used in multitest)
    println!(
        "{}*** Adding Pair Contracts (Juno) to Dummy DEX ***{}",
        YELLOW_UNDERLINE, WHITE
    );
    for pair in test_pairs {
        if show_output {
            println!("Saving pair: {} to {}", pair.token0, pair.token1);
        }
        let upsert_pair_msg = classes::pair_registry::ExecuteMsg::UpsertPair {
            token0: if pair.token0 == *"uloop" {
                override_test_cw20_address.clone()
            } else {
                pair.token0.clone()
            },
            token1: if pair.token1 == *"uloop" {
                override_test_cw20_address.clone()
            } else {
                pair.token1.clone()
            },
            routes: pair.routes,
        };
        let _ = router
            .execute_contract(
                Addr::unchecked(obi_owner.clone()),
                pair_registry_contract_addr.clone(),
                &upsert_pair_msg,
                &[],
            )
            .unwrap();
    }
    println!("{}...success{}", GREEN, WHITE);
    println!();

    (mocked_dummy_dex_contract_addrs, pair_registry_contract_addr)
}

pub fn obi_init_tests(
    router: &mut App,
    first_test: bool,
    default_asset: String,
) -> ObiTestConstants {
    if first_test {
        println!();
        println!("{} ██████╗ ██████╗ ██╗", BLUE);
        println!("{}██╔═══██╗██╔══██╗██║", BLUE);
        println!("{}██║   ██║██████╔╝██║", BLUE);
        println!("{}██║   ██║██╔══██╗██║", BLUE);
        println!("{}╚██████╔╝██████╔╝██║", BLUE);
        println!("{} ╚═════╝ ╚═════╝ ╚═╝", BLUE);
        println!();
        println!("{} Obi Account Factory Multi-Test", FORCED_WHITE);
    }

    let legacy_owner = Addr::unchecked("alice");
    let new_owner = Addr::unchecked("danielle");
    let obi_owner = Addr::unchecked("obi");

    // Store the codes
    let code_ids = get_code_ids(router);

    // Instantiate contracts
    let mut contract_addresses = instantiate_contracts(
        router,
        code_ids.clone(),
        obi_owner.clone(),
        first_test,
        first_test,
        default_asset,
    );

    // Use account creator to deploy a user account
    contract_addresses.user_entry = factory_deploy(
        router,
        legacy_owner.clone(),
        contract_addresses.clone(),
        code_ids.clone(),
    );

    println!(
        "Code id of user entry contract: {}",
        router
            .contract_data(&contract_addresses.user_entry.clone())
            .unwrap()
            .code_id
    );

    let user_account_address_res: UserAccountAddressResponse = router
        .wrap()
        .query_wasm_smart(
            contract_addresses.user_entry.clone(),
            &classes::msg_user_entry::QueryMsg::UserAccountAddress {},
        )
        .unwrap();
    contract_addresses.user_account =
        Addr::unchecked(user_account_address_res.user_account_address);
    println!(
        "Setting user account address to {}",
        contract_addresses.user_account
    );
    // Get the gatekeeper addresses instantiated by the account creator
    contract_addresses.read_gatekeepers(router);

    let o = ObiTestConstants {
        legacy_owner,
        new_owner,
        obi_owner,
        code_ids,

        // An authorized spend we'll be using for spend limit testing
        authorized_spender_daily: "bob".to_string(),
        authorized_spender_monthly: "carl".to_string(),

        beneficiary_full: "spouse".to_string(),
        beneficiary_drip: "child".to_string(),

        block_info: router.block_info(),
        contract_addresses,
    };

    // To test resets on recurring spend limits, we advance block_info's time
    // Let's advance some time right now for midnight/month day 1 testing
    router.set_block(BlockInfo {
        height: 12345u64, // currently used sometimes to detect we're in the multitester
        time: o.block_info.time.plus_seconds(12345u64), //advance into the day
        chain_id: o.block_info.chain_id.clone(),
    });

    o
}
