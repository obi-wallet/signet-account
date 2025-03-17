##User Entry Contract

`user_entry` is a simple passthrough contract to enable migration in contexts where native migration is not available.

Since user abstraction rule storage is confined to `user_state`, state migration is not required if the migration process is only updating other contracts.

The contract is instantiated with a `user_account_address`. Any native assets are actually held by the `user_entry` contract address, which does not change (and is, in contexts without native migration, not migratable). The current `user_account_address` can be queried so that administrative actions can be performed directly on it by the authorized owner. However, some messages must be passed through `user_entry`:

`ExecuteMsg::Execute` – since `user_entry` is the address which actually holds any user assets and is associated with the user in other application smart contracts, the final messages processed (approved or adjusted) by `user_account` and its gatekeepers must be added to a `Response` which is actually posted by `user_entry` itself. To do this, `ExecuteMsg::Execute` passes everything through to `user_account` as a `SubMsg` and then, upon a successful reply, adds any messages in the response to its own, resulting in their execution.

To migrate, `user_entry` should be pointed to the new `user_account_address`. A future version of `user_entry` will actually upgrade by completing the `account_creator`'s `new_account` process without requiring intermediary steps.