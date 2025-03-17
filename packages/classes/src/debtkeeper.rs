#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use uniserde::uniserde;
macros::cosmwasm_imports!(Addr, Coin, Uint256);

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct InstantiateMsg {
    pub asset_unifier_contract: String,
    pub asset_unifier_code_hash: String,
    pub user_account: String,
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
pub struct InstantiateMsg {
    pub asset_unifier_contract: String,
    pub user_account: String,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub enum ExecuteMsg {
    /// `UpdateLegacyOwner` allows us to change the legacy owner of the contract
    UpdateLegacyOwner {
        /// `new_legacy_owner` is the new contract owner
        new_legacy_owner: String,
    },
    /// `IncurDebt` is intended for use mid-transactions where
    /// the next action will not proceed if debt is not incurred.
    IncurDebt {
        /// Debt which will be added to the contract's debt
        additional_debt: Coin,
    },
    /// ClearDebt should only be callable by user_account contract,
    /// meaning that this is the only rulekeeper that must know
    /// its (immutable!) user account address.
    ClearDebt {
        /// Amount of debt that will be removed  from this contract's debt
        debt_to_clear: Coin,
    },
    /// One time only, set the user account address
    UpdateUserAccount {
        /// User account address
        user_account: Addr,
    },
}

/// QueryMsg
#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub enum QueryMsg {
    OutstandingDebt {},
    LegacyOwner {},
}

#[uniserde]
pub struct MigrateMsg {}

#[uniserde]
pub struct LegacyOwnerResponse {
    pub legacy_owner: Option<String>,
}

#[uniserde]
pub struct OutstandingDebtResponse {
    pub amount: Uint256,
    pub denom: String,
}
