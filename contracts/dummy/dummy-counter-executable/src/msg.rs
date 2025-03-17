#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
#[uniserde::uniserde]
pub struct InstantiateMsg {}

#[uniserde::uniserde]
pub enum ExecuteMsg {
    KobayashiMaru {
        captain: String,
        strategy: String,
    },
    DeepKobayashiMaru {
        captain: String,
        strategies: Vec<Substrategy>,
    },
}

#[uniserde::uniserde]
pub enum QueryMsg {
    CheaterDetected {},
}

#[uniserde::uniserde]
pub struct Substrategy {
    pub strategy: String,
    pub authorized: String,
}

#[uniserde::uniserde]
pub struct CheaterDetectedResponse {
    pub cheater_detected: bool,
}
