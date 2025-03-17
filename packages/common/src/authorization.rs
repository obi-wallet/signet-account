// Authorization
// ID for the specific actor and contract
// ID = actor + contract | actor:contract

// Allowed List

// message_name - The actual message type that we're trying to check permissions against
// `actor:contract:message_name` - index of the specific message we're trying to check allowed/denied actions
// `wasmaction_name` - {"<ACTION>" : { ...params... }}
// `actor:contract:message_name:wasmaction_name` = {
//     allow: Vec<ActionParam>,
//     deny: Vec<ActionParam>
// }

#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Addr, Binary);
use serde_json_value_wasm::{Map, Value};

// fields - the params that the wasmaction is able to have | Vec<Field>
// field: Field {
//   name: String, <- We *have* to specify the name of the field that we wish to allow/deny
//   value: Option<String> <- We *should* be allowed to specify the value we allow/deny
// }
#[uniserde::uniserde]
pub struct Authorization {
    /// Identifier is a non-zero number that represents the ID of the auth
    pub identifier: Option<u16>,
    pub actor: Option<Addr>,
    pub contract: Option<Vec<String>>, // needs to be String since can be e.g. an EVM contract address
    /// Message_name is the name of the message that we wish to match authorizations to
    /// MsgExecuteContract, MsgInstantiateContract
    pub message_name: Option<String>,
    /// wasmaction_name is the name of the action, e.g. "transfer" or "unstake"
    pub wasmaction_name: Option<String>,
    /// parameters for the above action
    /// FieldComp is assumed to be Equals if none
    #[allow(clippy::type_complexity)]
    pub fields: Option<Vec<(KeyValueOptions, Option<FieldComp>)>>,
    /// sessionkey functionality has been optimized to here;
    /// `actor` can be used along for a mostly unrestricted sessionkey
    pub expiration: u64,
}

#[uniserde::uniserde]
pub struct KeyValueOptions {
    pub key: String,
    pub allowed_values: Vec<StringOrBinary>,
}

#[uniserde::uniserde]
pub struct StringOrBinary {
    pub string: Option<String>,
    pub binary: Option<Binary>,
}

#[uniserde::uniserde]
pub enum FieldComp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    AnyOf,
    AnyMatchingObject,
}

#[uniserde::uniserde]
#[derive(Default)]
pub struct Authorizations {
    pub authorizations: Vec<(u16, Authorization)>,
}

impl Authorizations {
    pub fn find_by_id(&self, id: u16) -> Option<&Authorization> {
        self.authorizations
            .iter()
            .find(|a| a.1.identifier.unwrap_or(0u16) == id)
            .map(|a| &a.1)
    }

    pub fn filter_by_fields(&mut self, auth_to_match: &Authorization) -> Self {
        println!("filter_by_fields()");
        Self {
            authorizations: self
                .authorizations
                .iter()
                .filter(|a| {
                    if a.1.fields.is_some() {
                        if let Some(fields) = &auth_to_match.fields {
                            let parsed_fields_to_match: Map<String, Value> = fields
                                .iter()
                                .filter_map(|(k, _c)| match k.allowed_values.len() {
                                    1 => {
                                        k.allowed_values[0].string.as_ref().map(
                                            |string_value| (
                                                k.key.to_string(),
                                                Value::String(string_value.clone())
                                            )
                                        )
                                    },
                                    _ => None,  // Should be unreachable as messages won't have binary field values
                                })
                                .collect();
                            for (keyval, comp) in a.1.fields.clone().unwrap() {
                                let key = keyval.key;
                                let vals = keyval.allowed_values;
                                if parsed_fields_to_match.contains_key(&key) {
                                    return match comp.clone().unwrap_or(FieldComp::Equals) {
                                        FieldComp::AnyMatchingObject => {
                                            let any_matching_object = vals.iter().any(|desired_object_str| {
                                                match &desired_object_str.binary {
                                                    Some(auth_bin) => {
                                                        let desired_object: Value = serde_json_wasm::from_slice(auth_bin).unwrap();
                                                            vals.iter().any(|obj| {
                                                                if let Some(msg_bin) = &obj.binary {
                                                                    let de_bin: Value = serde_json_wasm::from_slice(msg_bin).unwrap();
                                                                    match de_bin.as_object() {
                                                                        Some(obj) => {
                                                                            desired_object.as_object().unwrap().iter().all(|(k, v)| {
                                                                                if v.as_str().unwrap() == "<ANY>" {
                                                                                    obj.contains_key(k)
                                                                                } else {
                                                                                    obj.get(k) == Some(v)
                                                                                }
                                                                            })
                                                                        }
                                                                        None => false,
                                                                    }
                                                                } else {
                                                                    false
                                                                }
                                                            })
                                                        },
                                                    None => false,  // Use binary for object matching, not string
                                                }
                                            });
                                            any_matching_object
                                        },
                                        FieldComp::Equals => {
                                            vals[0].string.clone().unwrap() == parsed_fields_to_match[&key]
                                        }
                                        FieldComp::NotEquals => {
                                            vals[0].string.clone().unwrap() != parsed_fields_to_match[&key]
                                        }
                                        FieldComp::AnyOf => {
                                            vals
                                            .iter()
                                            .map(|sob| Value::String(sob.string.clone().unwrap()))
                                            .collect::<Vec<Value>>()
                                            .contains(&parsed_fields_to_match[&key])
                                        },
                                        _ => match parsed_fields_to_match[&key].as_str() {
                                            None => false,
                                            Some(inner) => match inner.parse::<u64>() {
                                                Ok(inner_number) => match comp.unwrap() {
                                                    FieldComp::GreaterThan => {
                                                        inner_number
                                                            > vals[0].string.clone().unwrap().parse::<u64>().unwrap()
                                                    }
                                                    FieldComp::LessThan => {
                                                        inner_number
                                                            < vals[0].string.clone().unwrap().parse::<u64>().unwrap()
                                                    }
                                                    FieldComp::GreaterThanOrEqual => {
                                                        inner_number
                                                            >= vals[0].string.clone().unwrap().parse::<u64>().unwrap()
                                                    }
                                                    FieldComp::LessThanOrEqual => {
                                                        inner_number
                                                            <= vals[0].string.clone().unwrap().parse::<u64>().unwrap()
                                                    }
                                                    _ => false,
                                                },
                                                Err(_) => false,
                                            },
                                        },
                                    };
                                }
                            }
                            return true;
                        }
                        return true;
                    }
                    // by default we assume that we didn't match
                    false
                })
                .cloned()
                .collect(),
        }
    }

    pub fn filter_basic(&mut self, auth_to_match: &Authorization) -> Self {
        Self {
            authorizations: self
                .authorizations
                .iter()
                .filter(|a| {
                    // By default we assume that we do match ;)
                    let result = true;

                    // Actor filter isn't implemented here;
                    // used to filter queried AbstractionRules

                    // Only one contract in auth
                    if a.1
                        .contract
                        .clone()
                        .and_then(|f| {
                            if f.len() != 1 {
                                None
                            } else {
                                Some(f[0].clone())
                            }
                        })
                        .is_none()
                    {
                        return false;
                    }

                    // Check contract (if some) against allowed contracts
                    if auth_to_match.contract.is_some()
                        && !auth_to_match
                            .contract
                            .clone()
                            .unwrap()
                            .contains(&a.1.contract.clone().unwrap()[0])
                    {
                        return false;
                    }

                    // Are we also filtering by `message_name`?
                    if auth_to_match.message_name.is_some()
                        && auth_to_match.message_name != a.1.message_name
                    {
                        return false;
                    }

                    // Are we also filtering by `wasmaction_name`?
                    if auth_to_match.wasmaction_name.is_some()
                        && auth_to_match.wasmaction_name != a.1.wasmaction_name
                    {
                        return false;
                    }
                    // by default we assume that we matched!
                    result
                })
                .cloned()
                .collect(),
        }
    }

    /// `filter_authorization_by_msg` takes a filtered auth and a msg and try to see if the filtered auth
    /// contains the fields in the msg.
    ///
    /// `filter_auth` - a reference to a tuple containing the identifier and authorization.
    ///
    /// `msg` - a base64 string of the msg for us to filter against.
    pub fn filter_by_msg(&self, msg: &Binary) -> Self {
        Self {
            authorizations: self
                .authorizations
                .iter()
                .filter(|a| {
                    if a.1.fields.is_some() {
                        if let Ok(msg_to_val) = serde_json_wasm::from_slice::<Value>(msg) {
                            if let Some(val_to_obj) = msg_to_val.as_object() {
                                if val_to_obj.keys().len() == 1 {
                                    let obj_key = val_to_obj.keys().next().unwrap();
                                    let msg_fields = val_to_obj[obj_key].as_object().unwrap();
                                    for (keyval, comp) in a.1.fields.clone().unwrap() {
                                        let key = keyval.key;
                                        let vals = keyval.allowed_values;
                                        if !msg_fields.contains_key(&key) {
                                            return false;
                                        }
                                        if msg_fields.contains_key(&key) {
                                            println!("*** ***FILTER, checking if {:?} is any of {:?} in fields: {:?} *** ***", key, vals, msg_fields);
                                            println!("specified match type: {:?}", comp);
                                            if !match comp.clone().unwrap_or(FieldComp::Equals) {
                                                FieldComp::AnyMatchingObject => {
                                                    // Here we handle the AnyMatchingObject case.
                                                    // If the value is an array, iterates through it to find a match.
                                                    match msg_fields[&key].as_array() {
                                                        Some(array) => {
                                                            let any_matching_object = vals.iter().any(|desired_object_str| {
                                                                match &desired_object_str.binary {
                                                                    Some(auth_bin) => {
                                                                        let desired_object: Value = serde_json_wasm::from_slice(auth_bin).unwrap();
                                                                        array.iter().any(|obj| {
                                                                            if let Some(obj) = obj.as_object() {
                                                                                desired_object.as_object().unwrap().iter().all(|(k, v)| {
                                                                                    if v.as_str().unwrap() == "<ANY>" {
                                                                                        obj.contains_key(k)
                                                                                    } else {
                                                                                        obj.get(k) == Some(v)
                                                                                    }
                                                                                })
                                                                            } else {
                                                                                false
                                                                            }
                                                                        })
                                                                    },
                                                                    None => {
                                                                        false
                                                                    }
                                                                }
                                                            });
                                                            any_matching_object
                                                        },
                                                        None => {
                                                            false
                                                        }
                                                    }
                                                },
                                                FieldComp::Equals => {
                                                    println!("checking that {:?} is equal to {}", msg_fields[&key], vals[0].string.clone().unwrap());
                                                    msg_fields[&key] == vals[0].string.clone().unwrap()
                                                },
                                                FieldComp::NotEquals => msg_fields[&key] != vals[0].string.clone().unwrap(),
                                                FieldComp::AnyOf => {
                                                    vals
                                                    .iter()
                                                    .map(|sob| Value::String(sob.string.clone().unwrap()))
                                                    .collect::<Vec<Value>>()
                                                    .contains(&msg_fields[&key])
                                                }
                                                _ => {
                                                    match msg_fields[&key].as_str() {
                                                        None => false,
                                                        Some(inner) => {
                                                            match inner.parse::<u64>() {
                                                                Ok(inner_number) => {
                                                                    match comp.unwrap() {
                                                                        FieldComp::GreaterThan => inner_number > vals[0].string.clone().unwrap().parse::<u64>().unwrap(),
                                                                        FieldComp::LessThan => inner_number < vals[0].string.clone().unwrap().parse::<u64>().unwrap(),
                                                                        FieldComp::GreaterThanOrEqual => inner_number >= vals[0].string.clone().unwrap().parse::<u64>().unwrap(),
                                                                        FieldComp::LessThanOrEqual => inner_number <= vals[0].string.clone().unwrap().parse::<u64>().unwrap(),
                                                                        _ => false,
                                                                    }
                                                                },
                                                                Err(_) => false,
                                                            }
                                                        }
                                                    }
                                                }
                                            } { return false; }
                                        }
                                    }
                                    return true;
                                } else {
                                    return false; // multiple messages not handled here
                                }
                            }
                            return false;
                        }
                        false
                    } else {
                        true
                    }
                })
                .cloned()
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    // For unit tests
    #[uniserde::uniserde]
    pub enum TestMsg {
        KesselRun(TestExecuteMsg),
        KobayashiMaru(TestFieldsExecuteMsg),
    }

    #[uniserde::uniserde]
    pub struct TestExecuteMsg {
        pub parsecs: String,
    }

    #[uniserde::uniserde]
    pub struct TestFieldsExecuteMsg {
        pub recipient: String,
        pub strategy: String,
    }

    use super::*;
    macros::cosmwasm_imports!(to_binary);

    #[test]
    fn query_by_identifier() {
        let auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let auth_by_id = auths.find_by_id(1u16);
        assert!(auth_by_id.is_some());
    }

    #[test]
    fn query_by_wasmaction_name() {
        let mut auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let auth_by_wasm = auths.filter_basic(&Authorization {
            identifier: None,
            actor: None,
            contract: None,
            message_name: None,
            wasmaction_name: Some("kessel_run".to_string()),
            fields: None,
            expiration: 0,
        });

        assert!(auth_by_wasm.authorizations.len() == 1);
    }

    #[test]
    fn query_by_message_name() {
        let mut auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let auth_by_message = auths.filter_basic(&Authorization {
            identifier: None,
            actor: None,
            contract: None,
            message_name: Some("MsgExecuteContract".to_string()),
            wasmaction_name: None,
            fields: None,
            expiration: 0,
        });

        assert!(auth_by_message.authorizations.len() == 1);
    }

    #[test]
    fn query_by_fields() {
        let mut auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let auth_by_fields = auths.filter_by_fields(&Authorization {
            identifier: None,
            actor: None,
            contract: None,
            message_name: None,
            wasmaction_name: None,
            fields: Some(vec![
                (
                    KeyValueOptions {
                        key: "recipient".to_string(),
                        allowed_values: vec![StringOrBinary {
                            string: Some("god".to_string()),
                            binary: None,
                        }],
                    },
                    Some(FieldComp::Equals),
                ),
                (
                    KeyValueOptions {
                        key: "strategy".to_string(),
                        allowed_values: vec![StringOrBinary {
                            string: Some("refuse".to_string()),
                            binary: None,
                        }],
                    },
                    Some(FieldComp::Equals),
                ),
            ]),
            expiration: 0,
        });
        assert!(auth_by_fields.authorizations.len() == 1);
    }

    #[test]
    fn query_by_actor() {
        let mut auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let auth_by_actor = auths.filter_basic(&Authorization {
            identifier: None,
            actor: Some(Addr::unchecked("actor")),
            contract: None,
            message_name: None,
            wasmaction_name: None,
            fields: None,
            expiration: 0,
        });
        assert!(auth_by_actor.authorizations.len() == 1);
    }

    #[test]
    fn query_by_contract() {
        let mut auths: Authorizations = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let auth_by_contract = auths.filter_basic(&Authorization {
            identifier: None,
            actor: None,
            contract: Some(vec!["targetcontract".to_string()]),
            message_name: None,
            wasmaction_name: None,
            fields: None,
            expiration: 0,
        });

        assert!(auth_by_contract.authorizations.len() == 1);
    }

    #[test]
    fn check_transaction_wrong_fields() {
        let auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        // fails if strategy is wrong
        let msg = &TestMsg::KobayashiMaru(TestFieldsExecuteMsg {
            recipient: "god".to_string(),
            strategy: "assimilate".to_string(),
        });

        let check_msg = auths.filter_by_msg(&to_binary(&msg).unwrap());

        assert!(check_msg.authorizations.is_empty());
    }

    #[test]
    fn check_transaction_correct_fields() {
        let auths = Authorizations {
            authorizations: vec![(
                1u16,
                Authorization {
                    identifier: Some(1u16),
                    actor: Some(Addr::unchecked("actor")),
                    contract: Some(vec!["targetcontract".to_string()]),
                    message_name: Some("MsgExecuteContract".to_string()),
                    wasmaction_name: Some("kessel_run".to_string()),
                    fields: Some(vec![
                        (
                            KeyValueOptions {
                                key: "recipient".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("god".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                        (
                            KeyValueOptions {
                                key: "strategy".to_string(),
                                allowed_values: vec![StringOrBinary {
                                    string: Some("refuse".to_string()),
                                    binary: None,
                                }],
                            },
                            Some(FieldComp::Equals),
                        ),
                    ]),
                    expiration: 0,
                },
            )],
        };

        let msg = &TestMsg::KobayashiMaru(TestFieldsExecuteMsg {
            recipient: "god".to_string(),
            strategy: "refuse".to_string(),
        });

        let check_msg = auths.filter_by_msg(&to_binary(&msg).unwrap());

        assert!(check_msg.authorizations.len() == 1);
    }
}
