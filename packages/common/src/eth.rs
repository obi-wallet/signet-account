#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(ensure, StdError, StdResult, Uint256);
use tiny_keccak::{Hasher, Keccak};

use crate::common_error::ContractError;

#[uniserde::uniserde]
pub struct EthRawTx {
    pub nonce: Uint256,
    pub to: String,
    pub value: Uint256,
    pub gas_price: Uint256,
    pub gas: Uint256,
    pub data: Vec<u8>,
}

fn keccak256(message: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(message);
    hasher.finalize(&mut output);
    output
}

pub fn prep_hex_message(message: String) -> Vec<u8> {
    let valid_message = if let Some(stripped) = message.strip_prefix("0x") {
        stripped.to_string()
    } else {
        message
    };
    // Convert the message string from a hex string to a byte vector
    hex::decode(valid_message).unwrap()
}

pub fn prepend_and_or_hash(message: Vec<u8>, prepend: bool) -> Vec<u8> {
    let eth_message = if prepend {
        // Convert the length of the message in bytes to a string
        // Prepend the Ethereum-specific prefix and length to the message bytes
        let message_length_string = (message.len()).to_string();
        let mut eth_build_message =
            format!("\x19Ethereum Signed Message:\n{}", message_length_string).into_bytes();
        eth_build_message.extend(message);
        eth_build_message
    } else {
        message
    };

    keccak256(&eth_message).to_vec()
}

#[uniserde::uniserde]
pub struct EthUserOp {
    pub sender: String,
    pub nonce: Uint256,
    pub init_code: Vec<u8>,
    pub call_data: Vec<u8>,
    pub call_gas_limit: Uint256,
    pub verification_gas_limit: Uint256,
    pub pre_verification_gas: Uint256,
    pub max_fee_per_gas: Uint256,
    pub max_priority_fee_per_gas: Uint256,
    pub paymaster_and_data: Vec<u8>,
    pub signature: Vec<u8>,
}

impl EthUserOp {
    pub fn pad(&self, val: &[u8]) -> [u8; 32] {
        let mut bytes32: [u8; 32] = [0; 32];
        bytes32[32 - val.len()..].copy_from_slice(val);
        bytes32
    }

    // an independent function since the others were not hashing as expected
    pub fn get_user_op_hash(&self, entry_point: &str, chain_id: u32) -> Vec<u8> {
        let mut concatenated = Vec::with_capacity(320);

        concatenated.extend_from_slice(&self.pad(&hex::decode(self.sender.clone()).unwrap()));
        concatenated.extend_from_slice(&self.pad(&self.nonce.to_be_bytes()));
        concatenated.extend_from_slice(&keccak256(&self.init_code));
        concatenated.extend_from_slice(&keccak256(&self.call_data));
        concatenated.extend_from_slice(&self.pad(&self.call_gas_limit.to_be_bytes()));
        concatenated.extend_from_slice(&self.pad(&self.verification_gas_limit.to_be_bytes()));
        concatenated.extend_from_slice(&self.pad(&self.pre_verification_gas.to_be_bytes()));
        concatenated.extend_from_slice(&self.pad(&self.max_fee_per_gas.to_be_bytes()));
        concatenated.extend_from_slice(&self.pad(&self.max_priority_fee_per_gas.to_be_bytes()));
        concatenated.extend_from_slice(&keccak256(&self.paymaster_and_data));

        let mut encoded_outer: [u8; 96] = [0; 96];
        encoded_outer[..32].copy_from_slice(&keccak256(&concatenated));
        encoded_outer[44..64].copy_from_slice(&hex::decode(entry_point).unwrap());
        let offset = chain_id.to_be_bytes().len();
        let insertion = encoded_outer.len().saturating_sub(offset);
        encoded_outer[insertion..].copy_from_slice(&chain_id.to_be_bytes());

        keccak256(&encoded_outer).into()
    }

    pub fn make_fee_userop(&self, fee_divisor: Uint256, recipient: String) -> EthUserOp {
        // only for userops without fees built in (single call)
        let this_calldata = CallData::from_bytes(&self.call_data).unwrap().unwrap();
        if this_calldata.fee_amount.is_some() {
            panic!("Unreachable: trying to make_fee_userop when fee_amount is not None");
        }
        let mut fee_userop = self.clone();
        let mut fee_call_data = CallData::from_bytes(&self.call_data).unwrap().unwrap();
        let fee = fee_call_data.calculate_fee(fee_divisor).unwrap();
        fee_userop.nonce.checked_add(Uint256::from(1u128)).unwrap();
        fee_call_data.set_amount(fee);
        fee_call_data.set_recipient(recipient);
        let packed_fee_call_data = fee_call_data.repack(&self.call_data);
        fee_userop.call_data = packed_fee_call_data;
        fee_userop
    }
}

#[uniserde::uniserde]
pub struct CallData {
    pub contract: String,
    pub recipient: String,
    pub amount: Uint256,
    pub fee_recipient: Option<String>,
    pub fee_amount: Option<Uint256>,
    pub function_signatures: Vec<String>,
}

fn slice_to_array<T: Default + Clone + AsMut<[u8]>>(slice: &[u8]) -> T {
    let mut array: T = Default::default();
    let array_slice = array.as_mut();
    array_slice.copy_from_slice(slice);
    array
}

impl CallData {
    pub fn calculate_fee(&self, fee_divisor: Uint256) -> StdResult<Uint256> {
        Ok(self.amount.checked_div(fee_divisor).unwrap())
    }

    pub fn set_amount(&mut self, amount: Uint256) {
        self.amount = amount;
    }

    pub fn set_recipient(&mut self, recipient: String) {
        self.recipient = recipient;
    }

    pub fn from_bytes(call_data: &[u8]) -> Result<Option<Self>, ContractError> {
        // Check that call_data is at least 4 + 20 + 32 bytes
        if call_data.len() < 4 + 20 + 32 {
            return Ok(None);
        }

        // Function selector for "transfer(address,uint256)"
        let transfer_fn_selector: [u8; 4] = [182, 29, 39, 246];
        let multicall_fn_selector: [u8; 4] = [24, 223, 179, 199];

        // Check that the function selector matches
        // if transfer selector, then we'll send back a separate fee userop
        // if multicall, then we check the fee
        match &call_data[0..4] {
            val if val == transfer_fn_selector => {
                let token_contract: [u8; 20] = slice_to_array(&call_data[16..36]);
                let recipient: [u8; 20] = slice_to_array(&call_data[148..168]);
                let amount: [u8; 32] = slice_to_array(&call_data[168..200]);

                let token_contract_hex = hex::encode(token_contract);
                let recipient_hex = hex::encode(recipient);
                let amount_uint = Uint256::from_be_bytes(amount);
                println!("recipient {} gets amount {}", recipient_hex, amount_uint);

                let parsed_call_data = Self {
                    contract: token_contract_hex,
                    recipient: recipient_hex,
                    amount: amount_uint,
                    fee_recipient: None,
                    fee_amount: None,
                    function_signatures: vec![hex::encode(val)],
                };
                Ok(Some(parsed_call_data))
            }
            val if val == multicall_fn_selector => {
                // sample calldata
                // 18dfb3c7 // 0..4
                // 0000000000000000000000000000000000000000000000000000000000000040 //4..36
                // 00000000000000000000000000000000000000000000000000000000000000a0 //36..68
                // 0000000000000000000000000000000000000000000000000000000000000002 //68..100
                // 0000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2 //100..132
                // 0000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2 //132..164
                // 0000000000000000000000000000000000000000000000000000000000000002 //164..196
                // 0000000000000000000000000000000000000000000000000000000000000040 //196..228
                // 00000000000000000000000000000000000000000000000000000000000000c0 //228..260
                // 0000000000000000000000000000000000000000000000000000000000000044 //260..292
                // a9059cbb  //292..296
                // 0000000000000000000000008bb369366f14400327b86812fb1c4fa910a62389 //296..328
                // 0000000000000000000000000000000000000000000000000000000000000001 //328..360
                // 00000000000000000000000000000000000000000000000000000000 //360..388
                // 00000000000000000000000000000000000000000000000000000044 //388..416
                // a9059cbb  //416..424
                // 000000000000000000000000e423063e7ee6be8c5e482ce07a913710ecedc17d //424..456
                // 0000000000000000000000000000000000000000000000000000000000000001 //456..488
                // 00000000000000000000000000000000000000000000000000000000  //488..516
                let token_contract: [u8; 20] = slice_to_array(&call_data[112..132]);
                let token_contract_fee: [u8; 20] = slice_to_array(&call_data[144..164]);
                ensure!(
                    token_contract == token_contract_fee,
                    ContractError::Std(StdError::generic_err(
                        "Token fee must currently be paid in sent token"
                    ))
                );
                let recipient: [u8; 20] = slice_to_array(&call_data[308..328]);
                let fee_recipient: [u8; 20] = slice_to_array(&call_data[436..456]);
                let amount: [u8; 32] = slice_to_array(&call_data[328..360]);
                let fee_amount: [u8; 32] = slice_to_array(&call_data[456..488]);

                let token_contract_hex = hex::encode(token_contract);
                let recipient_hex = hex::encode(recipient);
                let fee_recipient_hex = hex::encode(fee_recipient);
                let amount_uint = Uint256::from_be_bytes(amount);
                let fee_amount = Uint256::from_be_bytes(fee_amount);
                println!("recipient {} gets amount {}", recipient_hex, amount_uint);

                let parsed_multicall_data = Self {
                    contract: token_contract_hex,
                    recipient: recipient_hex,
                    amount: amount_uint,
                    fee_recipient: Some(fee_recipient_hex),
                    fee_amount: Some(fee_amount),
                    function_signatures: vec![hex::encode(val)],
                };
                Ok(Some(parsed_multicall_data))
            }
            _ => Ok(None),
        }
    }

    pub fn repack(&self, original_call_data: &[u8]) -> Vec<u8> {
        let mut new_call_data = original_call_data.to_vec();
        let recipient_bytes = hex::decode(self.recipient.clone()).unwrap();
        let amount_bytes = self.amount.to_be_bytes();

        new_call_data[148..168].copy_from_slice(&recipient_bytes);
        new_call_data[168..200].copy_from_slice(&amount_bytes);

        new_call_data
    }
}

/* Multicall:
  sender: '0x12a2Fd1adA63FBCA7Cd9ec550098D48600D6dDc7',
  nonce: '0x1',
  initCode: '0x',
  callData: '0x18dfb3c7000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f20000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f20000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d600000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044a9059cbb000000000000000000000000c1d4f3dcc31d86a66de90c3c5986f0666cc52ce400000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000',
  callGasLimit: '0x189fa',
  verificationGasLimit: '0x243e2',
  preVerificationGas: '0xdbd4',
  maxFeePerGas: '0x18',
  maxPriorityFeePerGas: '0x2',
  paymasterAndData: '0xe93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064d9b24a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000308fc6c774d25c4db5197364dd174facbbf72871dae44c86929379765f8bc6705063cc07a45d3351c97a7703ff60a904135c756deb56a4cde775369316a19d9e1b',
  signature: '0x7088dd15b5e6f0725a9f762fd86644fc7cb6fef9fd37b5481821ad28ecb52b0144c48ed3eb4039693df527c5e66d9655cb4d4c680d8cf0ff7b3d747da75175101c'
*/
impl EthUserOp {
    pub fn dummy(inc_paymaster: bool, multicall: bool) -> Self {
        let call_data_vec = if multicall {
            // sending 1, fee 0.001
            hex::decode("18dfb3c7000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f20000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f20000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d600000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044a9059cbb000000000000000000000000c1d4f3dcc31d86a66de90c3c5986f0666cc52ce400000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000").unwrap()
        } else {
            hex::decode("b61d27f60000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d6000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000").unwrap()
        };
        Self {
            sender: "12a2Fd1adA63FBCA7Cd9ec550098D48600D6dDc7".to_string(),
            nonce: Uint256::from_u128(u128::from_str_radix("1", 16).unwrap()),
            init_code: vec![],
            call_data: call_data_vec,
            call_gas_limit: Uint256::from_u128(u128::from_str_radix("189fa", 16).unwrap()),
            verification_gas_limit: Uint256::from_u128(u128::from_str_radix("243e2", 16).unwrap()),
            pre_verification_gas: Uint256::from_u128(u128::from_str_radix("dbd4", 16).unwrap()),
            max_fee_per_gas: Uint256::from_u128(u128::from_str_radix("18", 16).unwrap()),
            max_priority_fee_per_gas: Uint256::from_u128(u128::from_str_radix("2", 16).unwrap()),
            paymaster_and_data: if inc_paymaster {
                hex::decode("e93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064d9b24a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000308fc6c774d25c4db5197364dd174facbbf72871dae44c86929379765f8bc6705063cc07a45d3351c97a7703ff60a904135c756deb56a4cde775369316a19d9e1b").unwrap()
            } else {
                vec![]
            },
            signature: vec![],
        }
    }

    // For some paymaster archs we're aiming for a result like this
    // @0x000: 0000000000000000000000009a7908627581072a5be468464c32ac8bf2239466 sender
    // @0x020: 0000000000000000000000000000000000000000000000000000000000000007 nonce
    // @0x040: 0000000000000000000000000000000000000000000000000000000000000160 offset of initCode
    // @0x060: 00000000000000000000000000000000000000000000000000000000000001a0 offset of callData
    // @0x080: 000000000000000000000000000000000000000000000000000000000000ab12 callGasLimit
    // @0x0a0: 000000000000000000000000000000000000000000000000000000000000de34 verificationGasLimit
    // @0x0c0: 00000000000000000000000000000000000000000000000000000000000000ef preVerificationGas
    // @0x0e0: 00000000000000000000000000000000000000000000000000000002540be400 maxFeePerGas
    // @0x100: 000000000000000000000000000000000000000000000000000000003b9aca00 maxPriorityFeePerGas
    // @0x120: 00000000000000000000000000000000000000000000000000000000000001e0 offset of paymasterAndData
    // @0x140: 0000000000000000000000000000000000000000000000000000000000000220 offset of signature
    // @0x160: 0000000000000000000000000000000000000000000000000000000000000004 length of initCode
    // @0x180: 1517c0de00000000000000000000000000000000000000000000000000000000 initCode
    // @0x1a0: 0000000000000000000000000000000000000000000000000000000000000004 length of callData
    // @0x1c0: ca11dada00000000000000000000000000000000000000000000000000000000 callData
    //
    // but for others...
    // function pack(UserOperation calldata userOp) internal pure returns (bytes memory ret) {
    //     address sender = getSender(userOp);
    //     uint256 nonce = userOp.nonce;
    //     bytes32 hashInitCode = calldataKeccak(userOp.initCode);
    //     bytes32 hashCallData = calldataKeccak(userOp.callData);
    //     uint256 callGasLimit = userOp.callGasLimit;
    //     uint256 verificationGasLimit = userOp.verificationGasLimit;
    //     uint256 preVerificationGas = userOp.preVerificationGas;
    //     uint256 maxFeePerGas = userOp.maxFeePerGas;
    //     uint256 maxPriorityFeePerGas = userOp.maxPriorityFeePerGas;
    //     bytes32 hashPaymasterAndData = calldataKeccak(userOp.paymasterAndData);
    //
    //     return abi.encode(
    //         sender, nonce,
    //         hashInitCode, hashCallData,
    //         callGasLimit, verificationGasLimit, preVerificationGas,
    //         maxFeePerGas, maxPriorityFeePerGas,
    //         hashPaymasterAndData
    //     );
    // }
    //
    // function hash(UserOperation calldata userOp) internal pure returns (bytes32) {
    //     return keccak256(pack(userOp));
    // }
    pub fn pack(&self) -> Vec<u8> {
        let mut keccak = Keccak::v256();
        let mut output = [0u8; 32];

        // Hash the packed EthUserOp
        keccak.update(&self.init_code);
        keccak.finalize(&mut output);

        let init_code_hash = output.to_vec();

        let mut keccak = Keccak::v256();

        // Hash the packed EthUserOp
        keccak.update(&self.call_data);
        keccak.finalize(&mut output);

        let call_data_hash = output.to_vec();

        let mut bytes: Vec<u8> = Vec::new();

        bytes.extend(self.sender.as_bytes());

        bytes.extend(&self.nonce.to_be_bytes());
        bytes.extend(init_code_hash);
        bytes.extend(call_data_hash);

        bytes.extend(&self.call_gas_limit.to_be_bytes());
        bytes.extend(&self.verification_gas_limit.to_be_bytes());
        bytes.extend(&self.pre_verification_gas.to_be_bytes());

        bytes.extend(&self.max_fee_per_gas.to_be_bytes());
        bytes.extend(&self.max_priority_fee_per_gas.to_be_bytes());
        bytes.extend(&self.paymaster_and_data);

        bytes
    }

    /// calculate the user op hash for signing
    pub fn hash(&self, entry_point: String, chain_id: Uint256) -> Vec<u8> {
        let mut keccak = Keccak::v256();
        let mut output = [0u8; 32];

        // Ensure user_op_hash is 32 bytes
        let user_op_packed = self.pack();
        keccak.update(&user_op_packed);
        keccak.finalize(&mut output);

        // Convert entry_point to a 20-byte value
        let entry_point_bytes = hex::decode(entry_point).unwrap();
        let mut entry_point_padded = [0u8; 20];
        entry_point_padded[..entry_point_bytes.len()].copy_from_slice(&entry_point_bytes);

        // Ensure chain_id is 32 bytes
        let chain_id_bytes = chain_id.to_be_bytes();
        let mut chain_id_padded = [0u8; 32];
        chain_id_padded[..chain_id_bytes.len()].copy_from_slice(&chain_id_bytes);

        let mut keccak = Keccak::v256();

        // Concatenate and hash the values
        keccak.update(&output);
        keccak.update(&entry_point_padded);
        keccak.update(&chain_id_padded);

        keccak.finalize(&mut output);

        let hash2 = output.to_vec();

        prepend_and_or_hash(hash2, true)
    }

    // Check if the destination contract is the one we expect
    pub fn is_tx_contract_equal_to(&self, expected_contract: Vec<u8>) -> bool {
        let ctt = self.call_data[16..36].to_vec();
        ctt == expected_contract
    }

    pub fn is_spend_less_than(&self, max_amount: Uint256) -> bool {
        println!("...checking spend...");
        println!(
            "checking that spend {} is <= max_amount {}",
            CallData::from_bytes(&self.call_data)
                .unwrap()
                .unwrap()
                .amount,
            max_amount
        );
        max_amount
            <= CallData::from_bytes(&self.call_data)
                .unwrap()
                .unwrap()
                .amount
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    const VALID_CONTRACT_ADDRESS: &str = "5cf29823ccfc73008fa53630d54a424ab82de6f2";
    const _TEST_SIGNING_KEY: &str =
        "6b6582a06ab08f38223a1e3b12ee8fc8a19efe690fb471dc151bb64588b23d96";
    const _RECIPIENT_ADDRESS: &str = "5D4DB1e7bae996F9F7d4E4a4165b6e9679B810d9";

    #[test]
    fn test_check_erc20_tx_invalid_contract() {
        let mut user_op = EthUserOp::dummy(false, false);
        user_op.call_data = hex::decode("b61d27f600000000000000000000000012a2fd1ada63fbca7cd9ec550098d48600d6ddc7000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d6000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000").unwrap();
        let contract_address = hex::decode(VALID_CONTRACT_ADDRESS).unwrap();
        // assert_eq!(user_op.get_erc20_spend(), Some(max_u256()));
        assert!(!user_op.is_tx_contract_equal_to(contract_address));
    }

    #[test]
    fn test_check_erc20_tx_valid_contract() {
        let mut user_op = EthUserOp::dummy(false, false);
        user_op.call_data = hex::decode("b61d27f60000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d6000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000").unwrap();
        let contract_address = hex::decode(VALID_CONTRACT_ADDRESS).unwrap();
        // assert_eq!(user_op.get_erc20_spend(), Some(max_u256()));
        assert!(user_op.is_tx_contract_equal_to(contract_address));
    }
}
