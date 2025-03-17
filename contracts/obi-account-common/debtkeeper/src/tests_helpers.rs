macros::cosmwasm_imports!(Coin, Env);

use classes::debtkeeper::InstantiateMsg;

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
        asset_unifier_contract: "test_asset_unifier_contract".to_string(),
        #[cfg(feature = "secretwasm")]
        asset_unifier_code_hash: "test_asset_unifier_code_hash".to_string(),
        user_account: "dummy_user_account".to_string(),
    }
}
