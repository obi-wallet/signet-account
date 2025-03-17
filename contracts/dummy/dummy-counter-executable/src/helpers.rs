#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(to_binary, Addr, CosmosMsg, StdResult, WasmMsg);

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[uniserde::uniserde]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }
}
