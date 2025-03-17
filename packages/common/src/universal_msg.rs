#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(CosmosMsg, Empty);
use crate::eth::EthUserOp;
// use crate::osmo_msg::OsmoMsg;

#[uniserde::uniserde]
pub enum UniversalMsg {
    Legacy(crate::legacy_cosmosmsg::CosmosMsg),
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    Secret(CosmosMsg),
    // osmosis has special message types; relevant for handling on target chains
    // Osmo(OsmoMsg),
    Eth(EthUserOp),
}

impl std::fmt::Display for UniversalMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
