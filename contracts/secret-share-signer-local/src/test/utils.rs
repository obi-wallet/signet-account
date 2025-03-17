#[cfg(test)]
pub mod tests {

    use std::str::FromStr;

    use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::sign::CompletedOfflineStage;
    use num_bigint::BigUint;

    use ec_curve::secp256k1::point::Secp256k1Point;
    use ec_curve::secp256k1::scalar::Secp256k1Scalar;
    use ec_curve::traits::{ECPoint, ECScalar};

    use crate::msg;

    impl msg::CompletedOfflineStageParts {
        pub fn from_value_hash(value: CompletedOfflineStage, user_entry_code_hash: String) -> Self {
            msg::CompletedOfflineStageParts {
                k_i: Secp256k1Scalar::from_slice(value.sign_keys.k_i.to_bytes().as_ref()).unwrap(),
                R: Secp256k1Point::from_xy(
                    &BigUint::from_str(value.R.x_coord().unwrap().to_string().as_str()).unwrap(),
                    &BigUint::from_str(value.R.y_coord().unwrap().to_string().as_str()).unwrap(),
                ),

                sigma_i: Secp256k1Scalar::from_slice(value.sigma_i.to_bytes().as_ref()).unwrap(),
                pubkey: Secp256k1Point::from_xy(
                    &BigUint::from_str(value.public_key().x_coord().unwrap().to_string().as_str())
                        .unwrap(),
                    &BigUint::from_str(value.public_key().y_coord().unwrap().to_string().as_str())
                        .unwrap(),
                ),
                user_entry_code_hash,
            }
        }
    }
}
