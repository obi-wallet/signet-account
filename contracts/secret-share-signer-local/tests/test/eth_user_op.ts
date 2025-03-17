import {ethers} from "ethers";

export interface EthUserOp {
  sender: string,
  nonce: string,
  initCode: string,
  callData: string,
  callGasLimit: string,
  verificationGasLimit: string,
  preVerificationGas: string,
  maxFeePerGas: string,
  maxPriorityFeePerGas: string,
  paymasterAndData: string,
}

export interface RustEthUserOp {
  sender: string,
  nonce: string,
  init_code: number[],
  call_data: number[],
  call_gas_limit: string,
  verification_gas_limit: string,
  pre_verification_gas: string,
  max_fee_per_gas: string,
  max_priority_fee_per_gas: string,
  paymaster_and_data: number[],
  signature: number[],
}

export function transformEthUserOp(ethUserOp: EthUserOp): RustEthUserOp {
  function hexToNumberArray(hexString: string): number[] {
    return Array.from(new Uint8Array(Buffer.from(hexString.slice(2), 'hex')));
  }

  return {
    sender: ethUserOp.sender.toLowerCase().startsWith('0x') ? ethUserOp.sender.slice(2) : ethUserOp.sender,
    nonce: parseInt(ethUserOp.nonce, 16).toString(10),
    init_code: hexToNumberArray(ethUserOp.initCode),
    call_data: hexToNumberArray(ethUserOp.callData),
    call_gas_limit: parseInt(ethUserOp.callGasLimit, 16).toString(10),
    verification_gas_limit: parseInt(ethUserOp.verificationGasLimit, 16).toString(10),
    pre_verification_gas: parseInt(ethUserOp.preVerificationGas, 16).toString(10),
    max_fee_per_gas: parseInt(ethUserOp.maxFeePerGas, 16).toString(10),
    max_priority_fee_per_gas: parseInt(ethUserOp.maxPriorityFeePerGas, 16).toString(10), // Fixed this line
    paymaster_and_data: hexToNumberArray(ethUserOp.paymasterAndData),
    signature: []
  };
}

export function createUserOperationHash(
  ethUserOp: EthUserOp,
  entryPointAddr: string,
  chainId: string
): string {
  const getUserOpHash = (): string => {
    const packed = ethers.AbiCoder.defaultAbiCoder().encode(
      [
        "address",
        "uint256",
        "bytes32",
        "bytes32",
        "uint256",
        "uint256",
        "uint256",
        "uint256",
        "uint256",
        "bytes32",
      ],
      [
        ethUserOp.sender,
        ethUserOp.nonce,
        ethers.keccak256(ethUserOp.initCode),
        ethers.keccak256(ethUserOp.callData),
        ethUserOp.callGasLimit,
        ethUserOp.verificationGasLimit,
        ethUserOp.preVerificationGas,
        ethUserOp.maxFeePerGas,
        ethUserOp.maxPriorityFeePerGas,
        ethers.keccak256(ethUserOp.paymasterAndData),
      ]
    );

    const enc = ethers.AbiCoder.defaultAbiCoder().encode(
      ["bytes32", "address", "uint256"],
      [ethers.keccak256(packed), entryPointAddr, chainId]
    );

    return ethers.keccak256(enc);
  };

  return getUserOpHash();
}
