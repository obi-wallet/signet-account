use crate::constants::{
    JUNO_MAINNET_AXLUSDC_IBC, OSMO_TEST_5_AXLUSDC_IBC, TERRA_MAINNET_AXLUSDC_IBC,
};

/// Takes in a `chain_id` and returns its corresponding IBC denom
pub fn get_axlusdc_ibc_denom(chain_id: String) -> String {
    match chain_id {
        actual if actual == *"juno-4" => JUNO_MAINNET_AXLUSDC_IBC.to_string(),
        actual if actual == *"osmo-test-5" => OSMO_TEST_5_AXLUSDC_IBC.to_string(),
        _ => TERRA_MAINNET_AXLUSDC_IBC.to_string(),
    }
}
