#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Addr, Binary, from_binary, to_binary);

use crate::{gatekeeper_common::GatekeeperType, rule::Rule};

use common::authorization::Authorization;

#[uniserde::uniserde]
pub struct AbstractionRules {
    pub rules: Vec<AbstractionRule>,
}

impl AbstractionRules {
    pub fn from_rule(rule: Rule, default_actor: Addr) -> Self {
        let (actor, gatekeeper_type) = match &rule {
            Rule::Spendlimit(params) => (
                Addr::unchecked(params.address.clone()),
                GatekeeperType::Spendlimit,
            ),
            Rule::Inheritance(params) => (
                Addr::unchecked(params.address.clone()),
                GatekeeperType::Inheritance,
            ),
            Rule::Allow(authorization) => (
                authorization.clone().actor.unwrap_or(default_actor),
                GatekeeperType::Allowlist,
            ),
            Rule::Block(authorization) => (
                authorization.clone().actor.unwrap_or(default_actor),
                GatekeeperType::Blocklist,
            ),
            Rule::Custom((actor, _rule)) => (Addr::unchecked(actor), GatekeeperType::Custom),
        };
        AbstractionRules {
            rules: vec![AbstractionRule {
                id: Some(1u16),
                actor,
                ty: gatekeeper_type,
                main_rule: rule,
            }],
        }
    }

    // useful helper for allow/blocklist work
    pub fn flatten_main_rules(&self) -> Vec<(u16, Rule)> {
        self.rules
            .iter()
            .map(|rule| (rule.id.unwrap(), rule.main_rule.clone()))
            .collect()
    }
}

#[uniserde::uniserde]
pub struct AbstractionRule {
    // idea is to be unique id; ignored when a param for AddAbstractionRule
    pub id: Option<u16>,
    pub actor: Addr,
    pub ty: GatekeeperType,
    pub main_rule: Rule,
}

pub struct AbstractionRuleBinary {
    // idea is to be unique id; ignored when a param for AddAbstractionRule
    pub id: Option<u16>,
    pub actor: Addr,
    pub ty: GatekeeperType,
    pub main_rule: Binary,
}

impl AbstractionRuleBinary {
    pub fn deserialize(&self) -> AbstractionRule {
        AbstractionRule {
            id: self.id,
            actor: self.actor.clone(),
            ty: self.ty.clone(),
            main_rule: from_binary(&self.main_rule).unwrap(),
        }
    }
}

impl AbstractionRule {
    pub fn serialize(&self) -> AbstractionRuleBinary {
        AbstractionRuleBinary {
            id: self.id,
            actor: self.actor.clone(),
            ty: self.ty.clone(),
            main_rule: to_binary(&self.main_rule).unwrap(),
        }
    }
}

impl Default for AbstractionRule {
    fn default() -> Self {
        AbstractionRule {
            id: None,
            actor: Addr::unchecked(""),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: None,
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: 0,
            }),
        }
    }
}
