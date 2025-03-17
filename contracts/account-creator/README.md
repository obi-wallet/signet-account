## Account Creator (Factory)

Using its `Config`, this creator instantiates new accounts.

Gatekeeper logic contracts (debt, message, sessionkey, and spendlimit) must be instantiated on chain before creating an account. A future version may instantiate contracts if their addresses are unknown.

When `ExecuteMsg::NewAccount` is called, the factory exectues several messages on itself, since new contract addresses are needed for subsequent actions.

`execute_new_account` calls the following messages:
1. `ExecuteMsg::InitUserAccount`, which creates a `user_account` contract and attaches the provided `user_state`, or, on `reply()`, instantiates a new `user_state` for the account. A `user_entry` contract is also instantiated on `reply()`. Contract addresses are saved to `ADDRESSES` as they come in from the `SubMsg` replies.
2. `ExecuteMsg::SetupUserState` instantiates the `user_state` contract (if needed) and then, on `reply()`, saves it to the `user_account` via its `ExecuteMsg::AttachUserAccount`.

Note: we may be instantiating an unused user_state unnecessarily. Integration tests don't thoroughly test migration yet.

The contract knows code IDs, code hashes, and common (shared) logic contract addresses thanks to them being stored in `CONFIG`. The `legacy_owner` can update these.