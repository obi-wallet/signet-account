//! Key refresh simulated
use std::collections::HashMap;

use cggmp_threshold_ecdsa::refresh::state_machine::KeyRefresh;
use curv::elliptic::curves::Secp256k1;
use serde::Deserialize;

use mpc_protocol::Parameters;

use crate::gg20::KeyShare;
use crate::gg2020_old::simulate::simulation::Simulation;
use crate::gg_2020::state_machine::keygen::LocalKey;

/// KeyRefreshItem
#[derive(Deserialize)]
pub enum KeyRefreshItem {
    /// KeyRefreshItem from existing key
    Existing {
        /// the existing party's key
        key: LocalKey<Secp256k1>,
        /// the party index to associate with the existing key
        updated_party_index: Option<u16>,
    },
    /// KeyRefreshItem to add a new party
    New {
        /// the new party's index
        party_index: u16,
    },
}

/// key_refresh_simulated implementation
pub fn key_refresh_simulated_impl(
    parameters: Parameters,
    key_refresh_items: Vec<KeyRefreshItem>,
) -> Result<
    Vec<KeyShare>,
    cggmp_threshold_ecdsa::refresh::state_machine::Error,
> {
    let new_t = parameters.threshold;
    let new_n = parameters.parties;

    let mut simulation = Simulation::<KeyRefresh>::new();
    let mut old_to_new = HashMap::new();
    let mut old_t = 0;
    for item in &key_refresh_items {
        match item {
            KeyRefreshItem::Existing {
                key,
                updated_party_index,
            } => {
                let new_party_index =
                    updated_party_index.unwrap_or(key.i);
                old_to_new.insert(key.i, new_party_index);
                old_t = key.t;
            }
            _ => {}
        }
    }
    for item in key_refresh_items {
        match item {
            KeyRefreshItem::Existing { key, .. } => {
                simulation.add_party(KeyRefresh::new(
                    Some(key),
                    None,
                    &old_to_new,
                    new_t,
                    new_n,
                    None,
                )?);
            }
            KeyRefreshItem::New { party_index } => {
                simulation.add_party(KeyRefresh::new(
                    None,
                    Some(party_index),
                    &old_to_new,
                    new_t,
                    new_n,
                    Some(old_t),
                )?);
            }
        }
    }

    let keys = simulation.run().unwrap();

    let key_shares: Vec<KeyShare> =
        keys.into_iter().map(|k| k.into()).collect();
    Ok(key_shares)
}
