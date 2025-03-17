use serde::{Deserialize, Serialize};

use ec_curve::secp256k1::point::Secp256k1Point;
use ec_curve::secp256k1::scalar::Secp256k1Scalar;
use ec_curve::secp256k1::CURVE_ORDER;
use ec_curve::traits::{ECPoint, ECScalar};

use crate::multi_party_ecdsa::Error;
use crate::multi_party_ecdsa::Error::InvalidSig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignatureRecid {
    pub r: Secp256k1Scalar,
    pub s: Secp256k1Scalar,
    pub recid: u8,
}

#[allow(non_snake_case)]
pub struct LocalSignature {
    pub r: Secp256k1Scalar,
    pub R: Secp256k1Point,
    pub s_i: Secp256k1Scalar,
    pub message: Vec<u8>,
    pub y: Secp256k1Point,
}

impl LocalSignature {
    #[allow(non_snake_case)]
    pub fn phase7_local_sig(
        k_i: &Secp256k1Scalar,
        message: &[u8],
        R: &Secp256k1Point,
        sigma_i: &Secp256k1Scalar,
        pubkey: &Secp256k1Point,
    ) -> LocalSignature {
        let m_fe = Secp256k1Scalar::from_slice(message).unwrap();
        // todo figure out if mod_floor is needed
        let r = R.x(); //.mod_floor(Secp256k1Scalar::group_order());
        let s_i = m_fe * k_i + &r * sigma_i;
        LocalSignature {
            r,
            R: R.clone(),
            s_i,
            message: message.to_vec(),
            y: pubkey.clone(),
        }
    }

    pub fn output_signature(self, s_vec: &[Secp256k1Scalar]) -> Result<SignatureRecid, Error> {
        let mut s = s_vec.iter().fold(self.s_i.clone(), |acc, x| acc + x);

        // todo figure out if mod_floor is needed
        let r = self.R.x(); //.mod_floor(Secp256k1Scalar::group_order());
                            // todo figure out if mod_floor is needed
        let ry = self.R.y(); //.mod_floor(Secp256k1Scalar::group_order());

        /*
         Calculate recovery id - it is not possible to compute the public key out of the signature
         itself. Recovery id is used to enable extracting the public key uniquely.
         1. id = R.y & 1
         2. if (s > curve.q / 2) id = id ^ 1
        */
        let is_ry_odd = !ry.is_even();
        let mut recid = if is_ry_odd { 1 } else { 0 };
        let s_tag = Secp256k1Scalar::from_slice(&CURVE_ORDER).unwrap() - s.clone();
        if s.is_high() {
            s = s_tag;
            recid ^= 1;
        }
        let sig = SignatureRecid { r, s, recid };
        let ver = verify(&sig, &self.y, &self.message).is_ok();
        if ver {
            Ok(sig)
        } else {
            Err(InvalidSig)
        }
    }
}

pub fn verify(sig: &SignatureRecid, y: &Secp256k1Point, message: &[u8]) -> Result<(), Error> {
    let b = sig.s.inv();
    let a = Secp256k1Scalar::from_slice(message).unwrap();
    let u1 = a * &b;
    let u2 = &sig.r * &b;

    let g = Secp256k1Point::generator();
    let gu1 = g * u1;
    let yu2 = y * &u2;
    // can be faster using shamir trick

    // todo figure out if mod_floor is needed
    if sig.r == (gu1 + yu2).x()
    /*.mod_floor(Secp256k1Scalar::from_slice(&CURVE_ORDER)))*/
    {
        Ok(())
    } else {
        Err(InvalidSig)
    }
}
