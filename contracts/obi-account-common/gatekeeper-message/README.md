## Message Gatekeeper

This logic contract can compare a message – currently Legacy (CosmosMsg), Secret (CosmosMsg with code hashes and no code admins), and Eth (EthUserOp) – with Allowlist or Blocklist rules and return whether a match is found or not.

CosmosMsg messages can match a combination of:
- `actor` (the authorized sender/signer) - user contract address here is shorthand for global block
- `contract` — the relevant smart contract address being executed by the message. Can be a vector of possible options
- `fields` - the parameters included with the message. Matching is rather advanced for CosmosMsg types, allowing ==, !=, >, <, AnyOf (any of a vector of possibilities), or deep JSON matching, whether a field = an arbitrary JSON structure, with <ANY> wildcards
- `message_name` – the message URL, such as `MsgExecuteContract`
- `wasmaction_name` - the top level action name for an execute action, such as `transfer`

`EthUserOp` matching is handled differently, with methods implemented on `CallData`.