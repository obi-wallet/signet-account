#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Attribute);

use crate::sourced_coin::SourcedCoins;

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
pub struct Source {
    pub contract_addr: String,
    pub query_msg: String,
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
pub struct Sources {
    pub sources: Vec<Source>,
}

impl Sources {
    pub fn append_sources(&mut self, sourced_coin: SourcedCoins) {
        for m in 0..sourced_coin.wrapped_sources.sources.len() {
            self.sources.push(Source {
                contract_addr: sourced_coin.wrapped_sources.sources[m]
                    .contract_addr
                    .clone(),
                query_msg: sourced_coin.wrapped_sources.sources[m].query_msg.clone(),
            });
        }
    }

    pub fn to_attributes(&self) -> Vec<Attribute> {
        let mut attributes: Vec<Attribute> = vec![];
        for source in self.sources.clone() {
            attributes.push(Attribute::new(source.contract_addr, source.query_msg));
        }
        attributes
    }
}
