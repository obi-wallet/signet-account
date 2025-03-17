# Obi Accounts, v1.2

See the [Obi Public Docs](https://obi-wallet.notion.site/Obi-Info-Docs-891c202333914587ae3e91f0e5430b58) for more information on Obi.

# Building and Testing

## Note on CosmWasm vs "SecretWasm"
Secret implementation of CosmWasm has some significant differences. The standard packages `cosmwasm-std` and `secret-cosmwasm-std` are used throughout, along with related packages such as `cw-serde` and `secret-toolkit`.

In order to avoid mass duplicate dependency declarations, a `cosmwasm_imports!()` macro imports from the appropriate package, depending on whether the feature `cosmwasm` is enabled or not. The feature `secretwasm` is enabled by default so that standard (reproducible) public docker optimizer images can be used without modification, so you will commonly see attributes such as `#[cfg(all(feature = 'secretwasm', not(feature = 'cosmwasm')))]`

Plain CosmWasm support is not very helpful for production deployments, since the `secret-share-signer` is currently the only signer implemented – backup signer options are on the roadmap. However, it does allow local multitest to be used, as Secret multitesting is not very well developed at this time.

## Multitest Integration Testing

Set `default = ["cosmwasm"]` feature for all contracts in order to run multitest integration tests without needing a network.

To run:
```
cd contracts/classes/account-creator
cargo test
```

This includes complete testing of the secret-share-signer.

For more verbose output, use `cargo test -- --show-output`.

## Compiling Secret Share Signer
secret-share-signer uses threshold cryptography and is thus reliant on cryptography libraries that some usual CosmWasm toolchain elements cannot handle. Some toolchains which work perfectly well for other contracts may fail here, so it is recommended to use the available Dockerized compile and optimization environment.

Use the special `make compile-optimized-reproducible` to successfully compile contracts/secret-share-signer to WASM that can be deployed on networks. Using cosmwasm optimizers or unoptimized WASM will result in errors. In particular, unoptimized WASM will throw `Wasm bytecode could not be deserialized. Deserialization error: "Unknown opcode 192"`

## MacOS notes
To run cargo tests, you may need:
`RUSTFLAGS="-L /opt/homebrew/lib" cargo test`

## Secretwasm Testnet Testing

The Passport signer requires `default = ["secretwasm"]` and uses its own tests in /contract/secret-share-signer/tests. These tests are not restricted to the signer contract; since the signer checks `can_execute`, for example, all other contracts are involved.

To compile, optimize, upload, instantiate, and run basic tests:
```
make clean && make compile-optimized-reproducible && cd contracts/secret-share-signer/tests && NODE_ENV=local yarn test
```

First check your `contracts/secret-share-signer/tests/.env.local`. Secret testnet values are available in `.env.testnet`.

## Quick Overview of Contracts
### For local/test networks
- dummy/dummy-counter-executable: used to provide an execute action (for tests such as message allowlist)

- contracts/obi-account/account-creator: a factory which creates user-account contracts with gatekeepers set up as requested
- contracts/obi-account/user-account: the account logic contract. Its contract is aware of its owner address, which can update itself, as well as connected gatekeepers
- contracts/obi-account/user-entry: the address which actually holds assets. It is a minimalistic wrapper around user-account so that the rest of the account can be migrated/updated in contexts without native migration capability
- contracts/obi-account/user-state: holds all user abstraction rules and other user state, such as last activity

### Attachable gatekeepers (can be shared, as they are logic only)
- contracts/obi-account/gatekeeper-message: a rulekeeper which can allow (or later block/delay) messages based on their actor, contract address, message name (such as "MsgDelegate"), WASM action name (such as "claim"), or any number of fields. Currently field checking can be ==, !=, ranges, AnyOf, and field == arbitrary JSON object. Ethereum User Ops currently only support contract == any of []
- contracts/obi-account/gatekeeper-sessionkey: a rulekeeper which tracks session keys (addresses), which can be manually destroyed, can expire at a set time, or can be used only a limited number of times. Sessionkeys can be limited by other rules such as spendlimits and message allow/block/delay lists
- contracts/obi-account/gatekeeper-spendlimit: a rulekeeper which tracks recurring spendlimits for addresses (optionally with a dormancy delay, for use cases such as inheritance) and checks whether a message exceeds a unified (e.g. USDC) spendlimit for the current time period
Gatekeepers don't hold user state; rules are held in user-state

### Signer
- contracts/signer: an unused old version of the signer which keeps a full private key and signs Ethereum UserOperations with it. It was used for development while secret-share-signer was in progress and is only included for reference if necessary
- contracts/secret-share-signer: uses threshold signing to finalize a UserOperation signature, after checking it with the user account to ensure the sender/signer is authorized to perform the transaction in question

## Migration
Native migration is not yet available on Secret. Once it is, it may or may not be enabled: we have a custom, simple upgrade process for most contracts.

The simple passthrough `user-entry` contract is used to enable easy migration of all code with its `ExecuteMsg:UpdateUserAccountAddress`, optionally preserving the current user state (abstraction rules and `last_activity`).

## Sample Multisend UserOperation
For testing of signature operations, here is a valid, signed multi-send UserOperation on Goerli testnet (5), with entry point 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789.

This sends 1 token and an additional fee payments of 0.001 token (assuming `fee_divisor` is set to 1000).

The key for the sender is `0x6b6582a06ab08f38223a1e3b12ee8fc8a19efe690fb471dc151bb64588b23d96`.

```
Signed UserOperation: {
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
}
UserOpHash: 0x3237715bcf5c565a7f45ef46bd8af9b616d5bc44224354b3b3a19c9d259c56cf
```