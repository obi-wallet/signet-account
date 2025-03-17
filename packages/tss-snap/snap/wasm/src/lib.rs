//! Webassembly utilities for the threshold signatures snap.
#![deny(missing_docs)]
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasm_bindgen::prelude::*;

use curv::elliptic::curves::secp256_k1::Secp256k1;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::{
    party_i::SignatureRecid, state_machine::keygen::LocalKey,
};

use web3_signature::Signature;
use web3_transaction::{
    eip1559::Eip1559TransactionRequest,
    types::{Address, U256},
    TypedTransaction,
};

extern crate wasm_bindgen;

#[doc(hidden)]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_log::init(wasm_log::Config::new(log::Level::Debug));
    log::info!(
        "WASM snap helpers module started {:?}",
        std::thread::current().id()
    );
}

/// Key share with a human-friendly label.
#[derive(Serialize, Deserialize)]
pub struct NamedKeyShare {
    label: String,
    share: KeyShare,
}

// TODO: share this KeyShare type with the main MPC webassembly module

/// Generated key share.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyShare {
    /// The secret private key.
    #[serde(rename = "localKey")]
    pub local_key: LocalKey<Secp256k1>,
    /// The public key.
    #[serde(rename = "publicKey")]
    pub public_key: Vec<u8>,
    /// Address generated from the public key.
    pub address: String,
}

/// Error thrown by the library.
#[derive(Debug, Error)]
pub enum Error {}

/// Export a key share as an encrypted web3 keystore.
#[wasm_bindgen(js_name = "exportKeyStore")]
pub fn export_key_store(
    address: JsValue,
    passphrase: JsValue,
    key_share: JsValue,
) -> Result<JsValue, JsError> {
    use web3_keystore::encrypt;
    let address: String = address.into_serde()?;
    let passphrase: String = passphrase.into_serde()?;
    let key_share: NamedKeyShare = key_share.into_serde()?;
    let key_share = serde_json::to_vec(&key_share)?;
    let mut rng = rand::thread_rng();
    let keystore = encrypt(&mut rng, key_share, &passphrase, Some(address))?;
    Ok(JsValue::from_serde(&keystore)?)
}

/// Import an encrypted web3 keystore as a named key share.
#[wasm_bindgen(js_name = "importKeyStore")]
pub fn import_key_store(
    passphrase: JsValue,
    key_store: JsValue,
) -> Result<JsValue, JsError> {
    use web3_keystore::{decrypt, KeyStore};
    let passphrase: String = passphrase.into_serde()?;
    let key_store: KeyStore = key_store.into_serde()?;
    let json_data = decrypt(&key_store, &passphrase)?;
    let key_share: NamedKeyShare = serde_json::from_slice(&json_data)?;
    Ok(JsValue::from_serde(&key_share)?)
}

/// Prepare an unsigned transaction.
///
/// Returns the transaction hash as bytes.
#[wasm_bindgen(js_name = "prepareUnsignedTransaction")]
pub fn prepare_unsigned_transaction(
    nonce: String,
    chain_id: u64,
    value: String,
    gas: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    from: JsValue,
    to: JsValue,
) -> Result<JsValue, JsError> {

    let nonce: U256 = nonce.parse()?;
    let value: U256 = value.parse()?;
    let gas: U256 = gas.parse()?;

    let max_fee_per_gas: U256 = max_fee_per_gas.parse()?;
    let max_priority_fee_per_gas: U256 = max_priority_fee_per_gas.parse()?;

    let tx = get_typed_transaction(
        nonce,
        chain_id,
        value,
        gas,
        max_fee_per_gas,
        max_priority_fee_per_gas,
        from,
        to,
    )?;

    let sighash = tx.sighash();

    Ok(JsValue::from_serde(&sighash)?)
}

/// Prepare a signed transaction.
///
/// Returns the hex-encoded raw transaction suitable for
/// a call to eth_sendRawTransaction().
#[wasm_bindgen(js_name = "prepareSignedTransaction")]
pub fn prepare_signed_transaction(
    nonce: String,
    chain_id: u64,
    value: String,
    gas: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    from: JsValue,
    to: JsValue,
    signature: JsValue,
) -> Result<JsValue, JsError> {

    let nonce: U256 = nonce.parse()?;
    let value: U256 = value.parse()?;
    let gas: U256 = gas.parse()?;

    let max_fee_per_gas: U256 = max_fee_per_gas.parse()?;
    let max_priority_fee_per_gas: U256 = max_priority_fee_per_gas.parse()?;

    let tx = get_typed_transaction(
        nonce,
        chain_id,
        value,
        gas,
        max_fee_per_gas,
        max_priority_fee_per_gas,
        from,
        to,
    )?;
    let signature: SignatureRecid = signature.into_serde()?;

    let r = signature.r.to_bytes().as_ref().to_vec();
    let s = signature.s.to_bytes().as_ref().to_vec();
    let v = signature.recid as u64;

    let signature = Signature {
        r: U256::from_big_endian(&r),
        s: U256::from_big_endian(&s),
        v,
    }
    .into_eip155(chain_id);

    let bytes = tx.rlp_signed(&signature);
    let encoded = format!("0x{}", hex::encode(&bytes.0));

    Ok(JsValue::from_serde(&encoded)?)
}

/// Helper to build a transaction.
pub fn get_typed_transaction(
    nonce: U256,
    chain_id: u64,
    value: U256,
    gas: U256,
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    from: JsValue,
    to: JsValue,
) -> Result<TypedTransaction, JsError> {
    let from: Vec<u8> = from.into_serde()?;
    let from = Address::from_slice(&from);

    let to: Vec<u8> = to.into_serde()?;
    let to = Address::from_slice(&to);

    // NOTE: must use an Eip1559 transaction
    // NOTE: otherwise ganache/ethers fails to
    // NOTE: parse the correct chain id!
    let tx: TypedTransaction = Eip1559TransactionRequest::new()
        .from(from)
        .to(to)
        .value(value)
        .max_fee_per_gas(max_fee_per_gas)
        .max_priority_fee_per_gas(max_priority_fee_per_gas)
        .gas(gas)
        .chain_id(chain_id)
        .nonce(nonce)
        .into();
    Ok(tx)
}
