use crate::rule::Rule;
use crate::storage_items::{ACCOUNT, USER_ACCOUNT_ADDRESS};
#[allow(unused_imports)]
use common::{
    authorization::Authorizations,
    common_error::{AccountError, AuthorizationError, ContractError},
    common_execute_reasons::CanExecute,
};
macros::cosmwasm_imports!(ensure);

/// Ensures that the provided address is an authorized address otherwise raises an error.
pub fn ensure_authorized(
    deps: Deps,
    _env: Env,
    sender: Addr,
    legacy_owner_override: Option<String>,
    call_location: String,
) -> Result<(), ContractError> {
    /* deps.api.debug(
        "checking that {} is authorized for contract {}",
        sender, env.contract.address
    ); */
    if is_user_account_contract(deps, sender.clone())? {
        return Ok(());
    }

    let account_string: String;
    // try to load the specified UserAccount from contract state
    let account_info = ACCOUNT.load(deps.storage);
    // if the fetched UserAccount is an error

    if account_info.is_err() {
        if let Some(address) = legacy_owner_override {
            // if legacy owner is a thing make that the account string
            account_string = address;
        } else {
            // grab the legacy owner from contract state
            account_string = LEGACY_OWNER.load(deps.storage)?.unwrap_or_default();
        }
    } else if let Ok(unwrapped_account) = account_info {
        // if no error
        // set the account string to the legacy owner from the fetched UserAccount
        account_string = unwrapped_account.legacy_owner.unwrap_or_default();
    } else {
        return Err(ContractError::Account(AccountError::CannotLoadAccount {}));
    }
    ensure!(
        account_string == sender,
        ContractError::Auth(AuthorizationError::UnauthorizedInfo(
            sender.to_string(),
            LEGACY_OWNER.load(deps.storage)?,
            call_location
        ))
    );

    Ok(())
}

#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(
    Addr,
    CosmosMsg,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdResult
);

#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use secret_toolkit::{serialization::Json, storage::Item};

#[uniserde::uniserde]
pub enum GatekeeperType {
    Allowlist,
    Blocklist,
    Custom,
    Debt,
    Delay,
    Inheritance, // uses spendlimit code
    Spendlimit,
}

/// This struct is a simple schema which defines things about a gatekeeper, including
/// what kind it is and how it should be used in order to approve/reject transactions,
/// so that the user_account contract code can remain as unopinionated as possible.
///
/// It's not really used so far, but is a future plan.
#[uniserde::uniserde]
pub struct GatekeeperInfo {
    /// gatekeeper type
    pub gatekeeper_type: GatekeeperType,
    /// *default* gatekeeper execution order priority (higher values run first)
    pub execution_priority: u8,
    /// *default* gatekeeper authority: a rider can only bypass a gatekeeper with a lower authority
    pub authority: u8,
    /// if true, spend type messages are automatically allowed (no consultation of allow message
    /// gatekeepers) if this gatekeeper returns true
    pub spend_rider: bool,
}

#[cfg(feature = "cosmwasm")]
pub const GATEKEEPER_INFO: Item<GatekeeperInfo> = Item::new("gatekeeper_info");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const GATEKEEPER_INFO: Item<GatekeeperInfo, Json> = Item::new(b"gatekeeper_info");

pub fn get_gatekeeper_info(deps: Deps) -> StdResult<GatekeeperInfo> {
    GATEKEEPER_INFO.load(deps.storage)
}

/// `InstantiateMsg` is used to create gatekeepers
#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct InstantiateMsg {
    pub eth_interpreter_address: String,
}

/// `InstantiateMsg` is used to create gatekeepers
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct InstantiateMsg {
    pub eth_interpreter_address: String,
    pub eth_interpreter_code_hash: String,
}

/// The stored contract legacy owner if it exists.
#[cfg(feature = "cosmwasm")]
pub const LEGACY_OWNER: Item<Option<String>> = Item::new("legacy_owner");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const LEGACY_OWNER: Item<Option<String>> = Item::new(b"legacy_owner");

/// Takes an address to check if they are the legacy owner
pub fn is_legacy_owner(deps: Deps, addy: Addr) -> StdResult<bool> {
    // Load contract's legacy owner
    let legacy_owner = LEGACY_OWNER.load(deps.storage);
    // return if stored legacy owner matches the supplied address
    match legacy_owner {
        Ok(Some(address_string)) => Ok(addy == address_string),
        Ok(None) => Ok(false),
        Err(_) => Ok(false),
    }
}

/// Update the legacy owner of the contract
pub fn update_legacy_owner(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    addy: Addr,
) -> Result<Response, ContractError> {
    ensure_authorized(deps.as_ref(), env, info.sender, None, macros::loc_string!())?;
    LEGACY_OWNER.save(deps.storage, &Some(addy.to_string()))?;
    Ok(Response::default().add_attribute("action", "update_legacy_owner"))
}

/// Takes an address and returns whether or not it is the user account contract.
pub fn is_user_account_contract(deps: Deps, address: Addr) -> StdResult<bool> {
    let user_account_contract = USER_ACCOUNT_ADDRESS.load(deps.storage);
    match user_account_contract {
        Ok(address_string) => Ok(address == address_string),
        Err(_) => Ok(false),
    }
}

#[uniserde::uniserde]
pub struct LegacyOwnerResponse {
    pub legacy_owner: String,
}

#[uniserde::uniserde]
pub struct CheckTxAgainstRuleResponse {
    // whether or not the tx is allowed, with reason
    pub can_execute: CanExecute,
    // an optional repay message if debt repayment is required
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    pub repay_msg: Option<CosmosMsg>,
    #[cfg(feature = "cosmwasm")]
    pub repay_msg: Option<LegacyCosmosMsg>,
    // for informational purposes, matching message authorization(s)
    pub authorizations: Option<Authorizations>,
    // implies permission for simple spend/transfer messages?
    pub spend_rider: bool,
    // any subrules that also must pass, even when can_execute is true
    pub subrules: Vec<Rule>,
    // an optional spendlimit reduction execute message to update user state
    pub reduce_spendlimit_msg: Option<CosmosMsg>,
}
