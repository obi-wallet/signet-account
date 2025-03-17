macros::cosmwasm_imports!(Coin, Env);
use classes::asset_unifier::InstantiateMsg;
use common::constants::TERRA_MAINNET_AXLUSDC_IBC;

use crate::tests_contract::LEGACY_OWNER;

pub fn get_test_instantiate_message(
    _env: Env,
    _starting_debt: Coin,
    obi_is_signer: bool,
) -> InstantiateMsg {
    let _signer2: String = if obi_is_signer {
        "juno17w77rnps59cnallfskg42s3ntnlhrzu2mjkr3e".to_string()
    } else {
        "signer2".to_string()
    };
    // instantiate the contract

    InstantiateMsg {
        default_asset_unifier: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
        legacy_owner: Some(LEGACY_OWNER.to_string()),
        home_network: "local".to_string(),
        pair_contract_registry: "pair_contract_registry".to_string(),
        #[cfg(feature = "secretwasm")]
        pair_contract_registry_code_hash: "dummy_hash".to_string(),
    }
}
