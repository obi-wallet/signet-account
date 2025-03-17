# Fee Manager

This simple contract stores the current fee rate (represented as `fee_divisor`, which divides the total send amount) and `fee_pay_address` for each `chain_id`.

Only the Passport signer (secret-share-signer) currently uses this, to verify that ERC20 User Operations include an appropriate fee. Querying `FeeDetails { chain_id: String}` returns:

```
FeeDetailsResponse {
    pub fee_pay_address: String,
    pub fee_divisor: u64,
}
```

The fee can be increased by the fee-manager contract owner, but can only be increased by up to 0.1% (absolute), and only once in any given 24 hour period.