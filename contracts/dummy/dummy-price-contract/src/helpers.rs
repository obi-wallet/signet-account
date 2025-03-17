#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Addr);

/* use crate::msg::{ExecuteMsg, QueryMsg}; */

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
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
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    /* pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }*/

    // Get Price
    /*
    pub fn simulation<Q, T, CQ>(&self, querier: &Q) -> StdResult<GetCountResponse>
    where
        Q: Querier,
        T: Into<String>,
        CQ: CustomQuery,
    {
        let msg = QueryMsg::Simulation { offer_asset: () } {};
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: GetCountResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res)
    } */
}
