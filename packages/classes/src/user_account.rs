macros::cosmwasm_imports!(
    ensure,
    to_binary,
    Addr,
    BankMsg,
    Coin,
    CosmosMsg,
    Deps,
    DepsMut,
    DistributionMsg,
    Env,
    QueryRequest,
    StakingMsg,
    StdError,
    StdResult,
    Uint128,
    Uint256,
    WasmMsg,
    WasmQuery
);
#[cfg(feature = "cosmwasm")]
use crate::universal_msg::OsmoMsg;
use crate::{
    debtkeeper::{OutstandingDebtResponse, QueryMsg as DebtQueryMsg},
    gatekeeper_common::{CheckTxAgainstRuleResponse, GatekeeperInfo, GatekeeperType},
    permissioned_address::CoinBalance,
    signers::Signers,
    sourced_coin::SourcedCoins,
    sources::Sources,
    submsgs::{PendingSubmsg, SubmsgType, WasmMsgType},
    user_state::AbstractionRules,
};
#[allow(unused_imports)]
use common::common_execute_reasons::CanExecuteReason::{
    Allowance, AllowanceAndAllowlist, AllowanceWithAllowlistAndReset,
    AllowanceWithBlanketAuthorizedToken, AllowanceWithReset, Beneficiary, BeneficiaryFullControl,
    BeneficiaryWithAllowlist, BeneficiaryWithAllowlistAndReset, BeneficiaryWithReset,
    NoFundsAndAllowlist, OwnerNoDelay, SessionkeyAsOwner, SessionkeyAsOwnerWithDebtButNoFundsSpent,
};
use common::common_execute_reasons::CannotExecuteReason::MultipleMessagesNotYetSupported;
use common::{
    coin256::Coin256,
    common_error::{AccountError, ContractError, FlowError, MessageError},
    common_execute_reasons::{CanExecute, CanExecuteReason, CannotExecuteReason, PendingReason},
    eth::CallData,
    legacy_cosmosmsg as LegacyMsg,
    universal_msg::UniversalMsg,
};
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
};

use DebtQueryMsg::OutstandingDebt;

#[cfg(feature = "cosmwasm")]
type GatekeeperInfoVec = Vec<(GatekeeperType, String)>;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
type GatekeeperInfoVec = Vec<(GatekeeperType, (String, String))>;
type MessageIsOk = bool;

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct UserAccount {
    /// Current legacy owner. None if ownership is handled elsewhere.
    pub legacy_owner: Option<String>,
    /// For lookup only
    pub evm_contract_address: Option<String>,
    pub evm_signing_address: Option<String>,
    /// Currently ineffective. Seconds of mandatory delay between
    /// ProposeUpdateOwner and ConfirmUpdateOwner.
    pub owner_updates_delay_secs: Option<u64>,
    /// Contract that manages debt
    pub debtkeeper_contract_addr: Option<String>,
    /// Contract that manages spend limits for permissioned addresses (shared)
    pub gatekeepers: Vec<String>,
    pub fee_pay_wallet: Option<String>,
    /// Stores multisig signers for recovery lookup.
    /// Increasingly this is used for verification, such as when signer
    /// verifies submitters of a query.
    pub signers: Signers,
    /// Unified user state (used by all contracts, settable only by user account)
    pub user_state_contract_addr: Option<String>,
    /// Asset unifier contract (shared)
    pub asset_unifier_contract_addr: Option<String>,
    /// True if this account was created for the user and is awaiting
    /// its initial owner
    pub magic_update: bool,
    /// Used for alternative verification (verifying signatures directly)
    pub nexthash: String,
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct UserAccount {
    /// Current legacy owner. None if ownership is handled elsewhere.
    pub legacy_owner: Option<String>,
    /// For lookup only
    pub evm_contract_address: Option<String>,
    pub evm_signing_address: Option<String>,
    /// Currently ineffective. Seconds of mandatory delay between
    /// ProposeUpdateOwner and ConfirmUpdateOwner.
    pub owner_updates_delay_secs: Option<u64>,
    /// Contract that manages debt
    pub debtkeeper_contract_addr: Option<String>,
    pub debtkeeper_code_hash: Option<String>,
    pub gatekeepers: Vec<(String, String)>,

    /// Address to send to when repaying fee debts (home chain only)
    pub fee_pay_wallet: Option<String>,
    /// Stores multisig signers for recovery lookup.
    /// Increasingly this is used for verification, such as when signer
    /// verifies submitters of a query.
    pub signers: Signers,
    /// Unified user state (used by all, settable only by user account)
    pub user_state_contract_addr: Option<String>,
    pub user_state_code_hash: Option<String>,
    /// Asset unifier contract (shared)
    pub asset_unifier_contract_addr: Option<String>,
    pub asset_unifier_code_hash: Option<String>,
    /// True if this account was created for the user and is awaiting
    /// its initial owner
    pub magic_update: bool,
    /// Used for alternative verification (verifying signatures directly)
    pub nexthash: String,
}

#[derive(Clone, Debug, PartialEq)]
enum EvaluationState {
    Rejected = 0,
    Start = 1,
    NoGlobalBlock = 2,
    MessageCheckOk = 3,
    SpendCheckOk = 4,
    BothChecksOk = 5,
}

/// `EvaluationStateGuard` is a state machine that enforces the legal
/// state transitions, preventing direct access even from within
/// the `CanExecuteDraft` class.
struct EvaluationStateGuard {
    evaluation_state: EvaluationState,
}

impl EvaluationStateGuard {
    /// Create a new default `EvaluationStateGuard` with state `Start`.
    pub fn new() -> Self {
        EvaluationStateGuard {
            evaluation_state: EvaluationState::Start,
        }
    }

    pub fn state(&self) -> &EvaluationState {
        &self.evaluation_state
    }

    /// Move the state back to `Rejected`, a permanent state.
    pub fn reject(&mut self) {
        self.evaluation_state = EvaluationState::Rejected;
    }

    /// Move the state to `NoGlobalBlock` if it's currently `Start`.
    /// Otherwise, as long as it's not `Rejected`, the state is unchanged.
    pub fn advance_to_no_global_block(&mut self) {
        self.evaluation_state = match self.evaluation_state {
            EvaluationState::Rejected => panic!("Invalid transition. Message has been rejected."),
            EvaluationState::Start => EvaluationState::NoGlobalBlock,
            EvaluationState::NoGlobalBlock => EvaluationState::NoGlobalBlock,
            EvaluationState::MessageCheckOk => EvaluationState::MessageCheckOk,
            EvaluationState::SpendCheckOk => EvaluationState::SpendCheckOk,
            EvaluationState::BothChecksOk => EvaluationState::BothChecksOk,
        };
    }

    /// Move the state to `MessageCheckOk` if it's currently `NoGlobalBlock`.
    /// If the state is `SpendCheckOk`, move it to `BothChecksOk`.
    /// Otherwise, as long as it's not `Rejected`, the state is unchanged.
    pub fn advance_to_message_check_ok(&mut self) {
        self.evaluation_state = match self.evaluation_state {
            EvaluationState::Rejected => panic!("Invalid transition. Message has been rejected."),
            EvaluationState::Start => {
                panic!("Invalid transition. Blocklist must be checked first.")
            }
            EvaluationState::NoGlobalBlock => EvaluationState::MessageCheckOk,
            EvaluationState::MessageCheckOk => EvaluationState::MessageCheckOk,
            EvaluationState::SpendCheckOk => EvaluationState::BothChecksOk,
            EvaluationState::BothChecksOk => EvaluationState::BothChecksOk,
        };
    }

    /// Move the state to `SpendCheckOk` if it's currently `NoGlobalBlock`.
    /// If the state is `MessageCheckOk`, move it to `BothChecksOk`.
    /// Otherwise, as long as it's not `Rejected`, the state is unchanged.
    pub fn advance_to_spend_check_ok(&mut self) {
        self.evaluation_state = match self.evaluation_state {
            EvaluationState::Rejected => panic!("Invalid transition. Message has been rejected."),
            EvaluationState::Start => {
                panic!("Invalid transition. Blocklist must be checked first.")
            }
            EvaluationState::NoGlobalBlock => EvaluationState::SpendCheckOk,
            EvaluationState::MessageCheckOk => EvaluationState::BothChecksOk,
            EvaluationState::SpendCheckOk => EvaluationState::SpendCheckOk,
            EvaluationState::BothChecksOk => EvaluationState::BothChecksOk,
        };
    }
}

/// `CanExecuteDraft` is a state machine that enforces the possible
/// states and state transitions for message evaluation.
struct CanExecuteDraft {
    can_execute: CanExecute,
    decisions: Vec<CanExecute>,
    evaluation_state: EvaluationStateGuard,
}

impl Display for CanExecuteDraft {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CanExecuteDraft {{ can_execute: {:?}, can_executes: {:?}, evaluation_state: {} }}",
            self.can_execute,
            self.decisions,
            self.evaluation_state.state().clone() as u8
        )
    }
}

impl CanExecuteDraft {
    /// Create a new `CanExecuteDraft` with default values:
    /// `CanExecute::No(CannotExecuteReason::NoMatchingRule)` and
    /// `EvaluationState::Start`.
    pub fn new() -> Self {
        CanExecuteDraft {
            can_execute: CanExecute::No(CannotExecuteReason::NoMatchingRule),
            decisions: vec![],
            evaluation_state: EvaluationStateGuard::new(),
        }
    }

    /// The result of a blocklist check is passed in here. If it's a
    /// confirmed NotBlocklisted result, we can advance the evaluation
    /// state to NoGlobalBlock.
    pub fn check_global(&mut self, result: CanExecute) {
        if result == CanExecute::Maybe(PendingReason::NotBlocklisted) {
            self.evaluation_state.advance_to_no_global_block();
            self.can_execute = result;
        }
        self.decisions.push(result);
        println!("check_global() done, current state: {}", self);
    }

    pub fn check_message(&mut self, result: CanExecute) {
        match result {
            CanExecute::Yes(_) | CanExecute::Maybe(_) => {
                self.evaluation_state.advance_to_message_check_ok();
                self.can_execute = result;
            }
            _ => {}
        }
        if result != CanExecute::No(CannotExecuteReason::NoMatchingRule) {
            self.decisions.push(result);
        }
        println!("check_message() done, current state: {}", self);
    }

    pub fn check_spend(&mut self, result: CanExecute) {
        match result {
            CanExecute::Yes(_) | CanExecute::Maybe(_) => {
                self.evaluation_state.advance_to_spend_check_ok();
                self.can_execute = result;
            }
            _ => {}
        }
        if result != CanExecute::No(CannotExecuteReason::NoMatchingRule) {
            self.decisions.push(result);
        }
        println!("check_spend() done, current state: {}", self);
    }

    pub fn clarify_reason(&mut self) {
        if self.decisions.contains(&CanExecute::Maybe(
            PendingReason::NoFundsPendingMessageCheck,
        )) && self.can_execute == CanExecute::Yes(CanExecuteReason::AllowanceAndAllowlist)
        {
            self.can_execute = CanExecute::Yes(CanExecuteReason::NoFundsAndAllowlist);
        }
    }

    pub fn harden_maybe(&mut self) {
        if matches!(self.can_execute, CanExecute::Maybe(_)) {
            if self.evaluation_state.state() == &EvaluationState::BothChecksOk {
                self.can_execute = *self
                    .decisions
                    .iter()
                    .find(|c| matches!(c, CanExecute::Yes(_)))
                    .unwrap_or(&CanExecute::Yes(CanExecuteReason::AllowanceAndAllowlist));
            } else {
                self.can_execute = *self
                    .decisions
                    .iter()
                    .find(|c| matches!(c, CanExecute::No(_)))
                    .unwrap_or(&CanExecute::No(CannotExecuteReason::NoMatchingRule));
            }
        }
    }

    pub fn reject(&mut self, result: CanExecute) {
        self.evaluation_state.reject();
        if matches!(result, CanExecute::No(_)) {
            self.can_execute = result;
        } else {
            self.can_execute = CanExecute::No(CannotExecuteReason::NoMatchingRule);
        }
        println!("reject() done, current state: {}", self);
    }
}

impl UserAccount {
    pub fn expend_magic_update(&mut self, new_owner: Addr) -> StdResult<()> {
        // one time operation only
        ensure!(
            self.magic_update,
            ContractError::Std(StdError::generic_err("magic update unavailable"))
        );
        self.legacy_owner = Some(new_owner.to_string());
        self.magic_update = false;
        Ok(())
    }
    /// can_execute checks whether the given address can execute the given messages.
    /// It returns a CanSpendResponse, which contains a CanExecute indicating whether
    /// the address can execute the message and a reason code indicating why or why not,
    /// for example, CanExecute::Yes(CanExecuteReason::Allowance)
    ///
    /// If the address is the owner, execute.
    /// If the address is not the owner, all gatekeepers need to be consulted.
    ///
    pub fn can_execute(
        &self,
        deps: Deps,
        env: Env,
        sender: String,
        msgs: Vec<UniversalMsg>,
    ) -> Result<CanExecuteResponse, ContractError> {
        // vec for future, but right now just first msg in it checked+attached
        if msgs.len() > 1 {
            return Ok(CanExecuteResponse {
                can_execute: CanExecute::No(MultipleMessagesNotYetSupported),
                reduce_spendlimit_msg: None,
            });
        }
        // if user is owner, we'll check debt and global allow/block/delay
        if let Some(addy) = self.legacy_owner.clone() {
            if addy == sender {
                deps.api
                    .debug("\x1b[3m\tCalling address is user account legacy owner.\x1b[0m");
                return self.can_owner_execute(deps, msgs[0].clone());
            }
        }

        deps.api
            .debug("\x1b[3m\tCalling address is not an owner.\x1b[0m");
        self.can_nonowner_execute(deps, env, sender, msgs[0].clone())
    }

    pub fn can_execute_mut(
        &mut self,
        deps: DepsMut,
        signatures: Vec<String>,
    ) -> Result<CanExecuteResponse, ContractError> {
        // we have two ways to prove owner. One is the signatures on the message;
        // the other (can_execute) is the typical verification against the legacy_owner, which
        // is designed to be a multisig address.
        if self.verify_signatures(deps, signatures)? {
            return Ok(CanExecuteResponse {
                can_execute: CanExecute::Yes(OwnerNoDelay),
                reduce_spendlimit_msg: None,
            });
        }

        Ok(CanExecuteResponse {
            can_execute: CanExecute::No(CannotExecuteReason::NoMatchingRule),
            reduce_spendlimit_msg: None,
        })
    }

    pub fn can_owner_execute(
        &self,
        _deps: Deps,
        _msg: UniversalMsg,
    ) -> Result<CanExecuteResponse, ContractError> {
        // currently safety of always being able to act as owner is
        // primary: debt repay and universal blocklist check should
        // be included here, but safely
        //
        // let _first_fund: Vec<Coin256>;
        // let _unused: bool;
        // (_unused, _first_fund) = self.extract_funds(msg, true)?;
        Ok(CanExecuteResponse {
            can_execute: CanExecute::Yes(OwnerNoDelay),
            reduce_spendlimit_msg: None,
        })
    }

    /// verify that >= threshold of signers have submitted signatures
    /// for the NEXTHASH value
    pub fn verify_signatures(
        &mut self,
        deps: DepsMut,
        signatures: Vec<String>,
    ) -> Result<bool, ContractError> {
        let current_hash = self.nexthash.clone();
        self.nexthash = common::keccak256hash((current_hash.clone() + &signatures[0]).as_bytes());

        // For now we assume threshold of half, floor rounded, minus 1.
        // In keeping with convention, threshold+1 signatures are required.
        deps.api.debug("calculating threshold");
        deps.api
            .debug(&format!("threshold is {}", self.signers.threshold));
        deps.api
            .debug(&format!("signatures length: {}", signatures.len()));
        let mut signatures_attached = 0;

        #[allow(unused_mut, unused_variables)]
        let mut signers = self.signers.clone();

        // remove the "0x" if it exists in `hash`
        #[allow(unused_variables)]
        let hash = if let Some(stripped) = current_hash.strip_prefix("0x") {
            stripped.to_string()
        } else {
            current_hash.to_string()
        };
        #[allow(unused_variables)]
        for signature in signatures {
            // try both recovery bytes
            // we're not sure these signers are on chain by this point, so we have to
            // approach it this way for now rather than asking the chain for the pubkeys
            let pubkey0: String;
            let pubkey1: String;
            #[allow(deprecated)]
            #[cfg(not(test))]
            {
                deps.api.debug("recovering pubkeys");
                pubkey0 = base64::encode(
                    deps.api
                        .secp256k1_recover_pubkey(
                            &hex::decode(hash.clone()).unwrap(),
                            &hex::decode(signature.clone()).unwrap(),
                            0,
                        )
                        .unwrap(),
                );
                pubkey1 = base64::encode(
                    deps.api
                        .secp256k1_recover_pubkey(
                            &hex::decode(hash.clone()).unwrap(),
                            &hex::decode(signature).unwrap(),
                            1,
                        )
                        .unwrap(),
                );
                deps.api.debug(&format!("pubkey0: {}", pubkey0));
                deps.api.debug(&format!("pubkey1: {}", pubkey1));
                deps.api.debug("done recovering pubkeys");
            }
            #[cfg(test)]
            {
                pubkey0 = "An9YoJRlklu1UeUuw/luOdbEEYoE+4d5OCVA0uzOwxG0".to_string();
                pubkey1 = "A71CrvmXmO30LpZKIt0IRp2alHcCcD7ldrJ2qBV3/5c/".to_string();
            }
            // in one line, find any signers, such as signers.signers[<ANY INDEX>].address, that match.
            // We will still use signers, so don't change the signers object - except that we
            // want to REMOVE the matching address.
            deps.api.debug(&format!(
                "looking in signers for pubkeys {} or {}",
                pubkey0, pubkey1
            ));
            #[cfg(not(test))]
            if signers
                .signers
                .iter()
                .any(|s| s.pubkey_base_64 == pubkey0 || s.pubkey_base_64 == pubkey1)
            {
                signers
                    .signers
                    .retain(|s| s.pubkey_base_64 != pubkey0 && s.pubkey_base_64 != pubkey1);
                signatures_attached += 1;
            }
            #[cfg(test)]
            {
                signatures_attached = 2;
            }

            if signatures_attached > self.signers.threshold {
                break;
            }
        }
        deps.api.debug(&format!(
            "done with verify_signatures(), returning {}",
            signatures_attached > self.signers.threshold
        ));
        Ok(signatures_attached > self.signers.threshold)
    }

    // TODO: zero spend must make `message_is_ok` false
    // otherwise unauthorized parties could do zero spends due to spend
    // and message both being marked OK unless blocklisted
    pub fn extract_funds(
        &self,
        msg: UniversalMsg,
        _owner_shortcut: bool,
    ) -> Result<(MessageIsOk, Vec<Coin256>), ContractError> {
        println!("extracting funds...");
        match msg {
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            UniversalMsg::Secret(secret_msg) => {
                match secret_msg.clone() {
                    CosmosMsg::Wasm(WasmMsg::Instantiate {
                        code_id: _,
                        msg: _,
                        funds: _,
                        label: _,
                        code_hash: _,
                        admin: _,
                    }) => Ok((false, vec![])),
                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: _,
                        msg: _,
                        funds,
                        code_hash: _,
                    }) => {
                        let mut processed_msg = PendingSubmsg {
                            msg: UniversalMsg::Secret(secret_msg),
                            contract_addr: None,
                            binarymsg: None,
                            funds: vec![],
                            ty: SubmsgType::Unknown,
                        };
                        processed_msg.add_funds(funds.to_vec());
                        let msg_type = processed_msg.process_and_get_msg_type();
                        if let SubmsgType::ExecuteWasm(WasmMsgType::Cw20Transfer) = msg_type {
                            return Ok((
                                true,
                                processed_msg
                                    .funds
                                    .into_iter()
                                    .map(Coin256::from_coin128)
                                    .collect(),
                            ));
                        }
                        // can't immediately pass but can proceed to fund checking
                        Ok((
                            false,
                            funds.into_iter().map(Coin256::from_coin128).collect(),
                        ))
                    }
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: _,
                        amount,
                    }) => Ok((
                        true,
                        amount.into_iter().map(Coin256::from_coin128).collect(),
                    )),
                    CosmosMsg::Staking(StakingMsg::Delegate {
                        amount,
                        validator: _,
                    }) => Ok((
                        true,
                        vec![amount]
                            .into_iter()
                            .map(Coin256::from_coin128)
                            .collect(),
                    )),
                    CosmosMsg::Staking(StakingMsg::Redelegate {
                        src_validator: _,
                        dst_validator: _,
                        amount,
                    }) => Ok((
                        true,
                        vec![amount]
                            .into_iter()
                            .map(Coin256::from_coin128)
                            .collect(),
                    )),
                    CosmosMsg::Staking(StakingMsg::Undelegate {
                        amount,
                        validator: _,
                    }) => Ok((
                        true,
                        vec![amount]
                            .into_iter()
                            .map(Coin256::from_coin128)
                            .collect(),
                    )),
                    CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                        validator: _,
                    }) => Ok((true, vec![])),
                    CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress { address: _ }) => {
                        Ok((false, vec![]))
                    }
                    CosmosMsg::Custom(_) => {
                        Err(ContractError::Msg(MessageError::InvalidCustomCosmosMsg {}))
                    }
                    _ => Err(ContractError::Msg(MessageError::InvalidCosmosMsg {})),
                }
            }
            UniversalMsg::Legacy(cosmos_msg) => {
                // Restore support by handling these messages: cosmwasm-std can't be imported,
                // or compiles fail, but the secret-cosmwasm-std equivalent has code_hash.
                // Ideally we have .into() support for secret<>cosmwasm messages too.
                match cosmos_msg.clone() {
                    LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
                        contract_addr: _,
                        msg: _,
                        funds,
                    }) => {
                        let mut processed_msg = PendingSubmsg {
                            msg: UniversalMsg::Legacy(cosmos_msg),
                            contract_addr: None,
                            binarymsg: None,
                            funds: vec![],
                            ty: SubmsgType::Unknown,
                        };
                        // we map since Coin may be a different type
                        processed_msg.add_funds(
                            funds
                                .clone()
                                .into_iter()
                                .map(|c| Coin {
                                    amount: Uint128::from(c.amount.u128()),
                                    denom: c.denom,
                                })
                                .collect::<Vec<Coin>>(),
                        );
                        let msg_type = processed_msg.process_and_get_msg_type();
                        if let SubmsgType::ExecuteWasm(WasmMsgType::Cw20Transfer) = msg_type {
                            return Ok((
                                true,
                                processed_msg
                                    .funds
                                    .into_iter()
                                    .map(|c| Coin256 {
                                        denom: c.denom,
                                        amount: Uint256::from_u128(c.amount.u128()),
                                    })
                                    .collect(),
                            ));
                        }
                        // can't immediately pass but can proceed to fund checking
                        Ok((
                            false,
                            funds
                                .into_iter()
                                .map(|c| {
                                    #[allow(clippy::useless_conversion)]
                                    Coin256::from_coin128(Coin {
                                        amount: Uint128::from(c.amount.u128()),
                                        denom: c.denom,
                                    })
                                    .into()
                                })
                                .collect::<Vec<Coin256>>(),
                        ))
                    }
                    LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Send {
                        to_address: _,
                        amount,
                    }) => Ok((
                        true,
                        amount
                            .into_iter()
                            .map(|c| {
                                #[allow(clippy::useless_conversion)]
                                Coin256::from_coin128(Coin {
                                    amount: Uint128::from(c.amount.u128()),
                                    denom: c.denom,
                                })
                                .into()
                            })
                            .collect::<Vec<Coin256>>(),
                    )),
                    #[cfg(feature = "staking")]
                    LegacyMsg::CosmosMsg::Staking(LegacyMsg::StakingMsg::Delegate {
                        amount,
                        validator: _,
                    }) => Ok((
                        true,
                        vec![amount]
                            .into_iter()
                            .map(|c| {
                                #[allow(clippy::useless_conversion)]
                                Coin256::from_coin128(Coin {
                                    amount: Uint128::from(c.amount.u128()),
                                    denom: c.denom,
                                })
                                .into()
                            })
                            .collect::<Vec<Coin256>>(),
                    )),
                    #[cfg(feature = "staking")]
                    LegacyMsg::CosmosMsg::Staking(LegacyMsg::StakingMsg::Redelegate {
                        src_validator: _,
                        dst_validator: _,
                        amount,
                    }) => Ok((
                        true,
                        vec![Coin256::from_coin128(Coin {
                            amount: Uint128::from(amount.amount.u128()),
                            denom: amount.denom,
                        })],
                    )),
                    #[cfg(feature = "staking")]
                    LegacyMsg::CosmosMsg::Staking(LegacyMsg::StakingMsg::Undelegate {
                        amount,
                        validator: _,
                    }) => Ok((
                        true,
                        vec![amount]
                            .into_iter()
                            .map(|c| {
                                #[allow(clippy::useless_conversion)]
                                Coin256::from_coin128(Coin {
                                    amount: Uint128::from(c.amount.u128()),
                                    denom: c.denom,
                                })
                                .into()
                            })
                            .collect::<Vec<Coin256>>(),
                    )),
                    #[cfg(feature = "staking")]
                    LegacyMsg::CosmosMsg::Distribution(
                        LegacyMsg::DistributionMsg::WithdrawDelegatorReward { validator: _ },
                    ) => Ok((true, vec![])),
                    #[cfg(feature = "staking")]
                    LegacyMsg::CosmosMsg::Distribution(
                        LegacyMsg::DistributionMsg::SetWithdrawAddress { address: _ },
                    ) => Ok((false, vec![])),
                    LegacyMsg::CosmosMsg::Custom(_) => {
                        Err(ContractError::Msg(MessageError::InvalidCustomCosmosMsg {}))
                    }
                    _ => Err(ContractError::Msg(MessageError::InvalidCosmosMsg {})),
                }
            }
            // https://docs.osmosis.zone/osmosis-core/classes/gamm/#messages
            // todo: move to poolmanager message imports instead
            // UniversalMsg::Osmo(osmo_msg) => match osmo_msg {
            //     OsmoMsg::SwapExactAmountIn(msg) => Ok((
            //         true,
            //         vec![Coin256 {
            //             denom: msg.token_in.clone().unwrap().denom,
            //             amount: Uint256::from(
            //                 msg.token_in.unwrap().amount.parse::<u128>().unwrap(),
            //             ),
            //         }],
            //     )),
            //     // tbd whether we want things like token_out_min here to
            //     // be limited. Otherwise should be ridered
            //     OsmoMsg::ExitPool(_msg) => Ok((true, vec![])),
            //     // temporarily using token_out as funds until LP share price
            //     // discovery is supported
            //     OsmoMsg::ExitSwapExternAmountOut(msg) => Ok((
            //         true,
            //         vec![Coin256 {
            //             denom: msg.token_out.clone().unwrap().denom,
            //             amount: Uint256::from(
            //                 msg.token_out.unwrap().amount.parse::<u128>().unwrap(),
            //             ),
            //         }],
            //     )),
            //     // temporarily using token_out as funds until LP share price
            //     // discovery is supported
            //     OsmoMsg::ExitSwapShareAmountIn(msg) => Ok((
            //         true,
            //         vec![Coin256 {
            //             denom: msg.token_out_denom,
            //             amount: Uint256::from(msg.token_out_min_amount.parse::<u128>().unwrap()),
            //         }],
            //     )),
            //     OsmoMsg::JoinPool(msg) => Ok((
            //         msg.token_in_maxs != vec![],
            //         // `token_in_maxs` is only used if present! Otherwise, user's balance
            //         // is only constraint. If spend limited we must have this, so spend
            //         // limit message rider is *false* if none included
            //         msg.token_in_maxs
            //             .into_iter()
            //             .map(|coin| Coin256 {
            //                 denom: coin.denom,
            //                 amount: Uint256::from(coin.amount.parse::<u128>().unwrap()),
            //             })
            //             .collect(),
            //     )),
            //     OsmoMsg::JoinSwapExternAmountIn(msg) => Ok((
            //         true,
            //         vec![Coin256 {
            //             denom: msg.token_in.clone().unwrap().denom,
            //             amount: Uint256::from(
            //                 msg.token_in.unwrap().amount.parse::<u128>().unwrap(),
            //             ),
            //         }],
            //     )),
            //     OsmoMsg::JoinSwapShareAmountOut(msg) => Ok((
            //         true,
            //         vec![Coin256 {
            //             denom: msg.token_in_denom,
            //             amount: Uint256::from(msg.token_in_max_amount.parse::<u128>().unwrap()),
            //         }],
            //     )),
            //     // temporarily using token_out for spend amount
            //     OsmoMsg::SwapExactAmountOut(msg) => Ok((
            //         true,
            //         vec![Coin256 {
            //             denom: msg.routes[0].token_in_denom.clone(),
            //             amount: Uint256::from(
            //                 msg.token_out
            //                     .clone()
            //                     .unwrap()
            //                     .amount
            //                     .parse::<u128>()
            //                     .unwrap(),
            //             ),
            //         }],
            //     )),
            //     _ => Err(ContractError::Msg(MessageError::InvalidOsmosisMsg {})),
            // },
            UniversalMsg::Eth(tx) => {
                println!("Matched on UniversalMsg::Eth");
                if let Some(call_data) = CallData::from_bytes(&tx.call_data)? {
                    println!(
                        "Call data funds are {} {}",
                        call_data.amount,
                        call_data.clone().contract
                    );
                    Ok((
                        true,
                        vec![Coin256 {
                            denom: call_data.clone().contract,
                            amount: call_data.amount,
                        }],
                    ))
                } else {
                    Ok((false, vec![]))
                }
            }
        }
    }

    fn get_gatekeeper_infos(&self, deps: Deps) -> Result<GatekeeperInfoVec, ContractError> {
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let mut gatekeeper_infos: GatekeeperInfoVec = vec![];
        #[cfg(feature = "cosmwasm")]
        let mut gatekeeper_infos: GatekeeperInfoVec = vec![];
        for lookup in &self.gatekeepers {
            let info_query_res: GatekeeperInfo = deps.querier.query_wasm_smart(
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                lookup.1.clone(),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                lookup.0.clone(),
                #[cfg(feature = "cosmwasm")]
                lookup,
                &crate::msg_gatekeeper_message::QueryMsg::GatekeeperInfo {},
            )?;
            gatekeeper_infos.push((info_query_res.gatekeeper_type, lookup.clone()));
        }
        Ok(gatekeeper_infos)
    }

    pub fn can_nonowner_execute(
        &self,
        deps: Deps,
        env: Env,
        sender: String,
        msg: UniversalMsg,
    ) -> Result<CanExecuteResponse, ContractError> {
        // The `CanExecuteDraft` class forces defaults and
        // guards advancement of evaluation state
        let mut can_execute_draft = CanExecuteDraft::new();

        // get AbstractionRules for sender (if any) and then for the
        // user address (i.e. global allow/block rules)
        let user_account = env.contract.address.to_string();
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let user_account_code_hash = env.contract.code_hash.clone();

        // get attached gatekeepers
        let gatekeeper_infos = self.get_gatekeeper_infos(deps)?;

        // check global rules (currently just blocklist)
        let global_result = self.check_global_rules(deps, env, &msg, &sender, &gatekeeper_infos)?;
        can_execute_draft.check_global(global_result.can_execute);
        if matches!(global_result.can_execute, CanExecute::No(_)) {
            return Ok(global_result);
        }

        // get AbstractionRules for this specific sender
        let sender_rules: AbstractionRules = deps
            .querier
            .query_wasm_smart(
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                self.user_state_code_hash.clone().unwrap_or_default(),
                self.user_state_contract_addr
                    .clone()
                    .ok_or(ContractError::Account(
                        AccountError::UserStateContractAddressNotSet(macros::loc_string!()),
                    ))?,
                &crate::msg_user_state::QueryMsg::AbstractionRules {
                    actor: Some(deps.api.addr_validate(&sender)?),
                    ty: vec![],
                },
            )
            .map_err(|e| {
                ContractError::Std(StdError::generic_err(format!(
                    "{}, {}",
                    macros::loc_string!(),
                    e
                )))
            })?;

        // no global allow is permitted in this version! Global allows can only
        // apply to accounts with other rules. We must have at least 1 matching rule
        // for sender.
        if sender_rules.rules.is_empty() {
            can_execute_draft.reject(CanExecute::No(CannotExecuteReason::NoMatchingRule));
            return Ok(CanExecuteResponse {
                can_execute: CanExecute::No(CannotExecuteReason::NoMatchingRule),
                reduce_spendlimit_msg: None,
            });
        }

        let reduce_spendlimit_msg: Option<CosmosMsg> = None;

        deps.api.debug(&format!(
            "\x1b[3m\tAnalyzing message: \x1b[90m{:#?}\x1b[0m",
            msg
        ));

        // `extract_funds` will set `message_is_ok` to `true` if the message
        // is a type that should be auto-allowed with a spendlimit. Blocklist was already
        // checked above. If no funds are found, `spend_is_ok` is true – yet spendlimit
        // must still be checked, since allowing permissioned addresses to trigger the
        // transfer etc. functions of unknown assets is not safe.
        let (spendlimit_rider, funds) = self.extract_funds(msg.clone(), false)?;
        if spendlimit_rider {
            can_execute_draft.check_message(CanExecute::Maybe(
                PendingReason::SpendlimitMessageRiderPendingSpendCheck,
            ))
        }
        // after this point, spendlimit_rider is used again only to clarify reason output

        if funds.is_empty() {
            deps.api
                .debug("\x1b[3m\tNo funds used or attached in this transaction.\x1b[0m");
            // we can shortcut the spendlimit check if this is a message-only check
            can_execute_draft
                .check_spend(CanExecute::Maybe(PendingReason::NoFundsPendingMessageCheck))
        } else {
            deps.api.debug("\x1b[3m\tYes, this TX uses funds.\x1b[0m");
        }

        // In order to pass, we must have:
        // `message_is_ok`: not blacklisted; explicitly allowed or covered by a rider
        // `spend_is_ok`: either no funds spent or fund spend is allowed

        // TODO: add rule hinting when submitting tx for signing/executing
        for rule in sender_rules.rules {
            // look up the appropriate gatekeeper in gatekeeper_infos by the rule.ty
            // doesn't support multiple gatekeepers of the same type
            let check_type = match rule.ty {
                GatekeeperType::Inheritance => GatekeeperType::Spendlimit,
                _ => rule.ty.clone(),
            };
            let maybe_address = gatekeeper_infos
                .clone()
                .into_iter()
                .find_map(|(ty, address)| {
                    if ty == check_type {
                        Some(address)
                    } else {
                        None
                    }
                });
            if let Some(address) = maybe_address {
                deps.api.debug(&format!("Examining rule: {:#?}", rule));
                deps.api.debug(&format!("Inter-contract query: \x1b[1;34mUser Account\x1b[0m querying \x1b[1;34m{:?}\x1b[0m", rule.ty));
                let gatekeeper_res: StdResult<CheckTxAgainstRuleResponse> =
                    deps.querier.query_wasm_smart(
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        address.1.clone(),
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        address.0.clone(),
                        #[cfg(feature = "cosmwasm")]
                        address,
                        &crate::msg_gatekeeper_message::QueryMsg::CheckTxAgainstRule {
                            msg: msg.clone(),
                            sender: sender.clone(),
                            funds: funds.clone(),
                            rule: rule.main_rule.clone(),
                            rule_id: rule.id.unwrap(),
                            user_account: user_account.clone(),
                            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                            user_account_code_hash: Some(user_account_code_hash.clone()),
                            #[cfg(feature = "cosmwasm")]
                            user_account_code_hash: None,
                        },
                    );
                println!("received CheckTxAgainstRuleResponse: {:#?}", gatekeeper_res);
                if let Ok(g_r) = gatekeeper_res {
                    match &check_type {
                        GatekeeperType::Allowlist => {
                            can_execute_draft.check_message(g_r.can_execute);
                        }
                        GatekeeperType::Spendlimit => {
                            can_execute_draft.check_spend(g_r.can_execute);
                        }
                        _ => {
                            return Err(ContractError::Flow(FlowError::MismatchedRuleTypes(
                                format!("{:?}", check_type.clone()),
                            )))
                        }
                    }
                }
            }
        }
        can_execute_draft.harden_maybe();
        can_execute_draft.clarify_reason();
        deps.api.debug(&format!("{}", can_execute_draft));
        Ok(CanExecuteResponse {
            can_execute: can_execute_draft.can_execute,
            reduce_spendlimit_msg,
        })
    }

    fn check_global_rules(
        &self,
        deps: Deps,
        env: Env,
        msg: &UniversalMsg,
        sender: &str,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] gatekeeper_infos: &[(
            GatekeeperType,
            (String, String),
        )],
        #[cfg(all(feature = "cosmwasm"))] gatekeeper_infos: &Vec<(GatekeeperType, String)>,
    ) -> Result<CanExecuteResponse, ContractError> {
        // get global Abstraction rules (check for blocks)
        let global_rules: StdResult<AbstractionRules> = deps.querier.query_wasm_smart(
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            self.user_state_code_hash
                .clone()
                .ok_or(ContractError::Account(
                    AccountError::UserStateCodeHashNotSet(macros::loc_string!()),
                ))?,
            self.user_state_contract_addr
                .clone()
                .ok_or(ContractError::Account(
                    AccountError::UserStateContractAddressNotSet(macros::loc_string!()),
                ))?,
            &crate::msg_user_state::QueryMsg::AbstractionRules {
                actor: Some(env.contract.address.clone()),
                ty: vec![GatekeeperType::Blocklist],
            },
        );

        if let Err(e) = global_rules {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "{}, {}",
                macros::loc_string!(),
                e
            ))));
        }
        for rule in global_rules?.rules {
            match rule.ty {
                GatekeeperType::Blocklist => {
                    let maybe_address = gatekeeper_infos
                        .iter()
                        .filter_map(
                            |(ty, address)| if ty == &rule.ty { Some(address) } else { None },
                        )
                        .next();
                    if let Some(address) = maybe_address {
                        let gatekeeper_res: CheckTxAgainstRuleResponse =
                            deps.querier.query_wasm_smart(
                                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                                address.1.clone(),
                                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                                address.0.clone(),
                                #[cfg(feature = "cosmwasm")]
                                address,
                                &crate::msg_gatekeeper_message::QueryMsg::CheckTxAgainstRule {
                                    msg: msg.clone(),
                                    sender: sender.to_string(),
                                    funds: vec![], // irrelevant for now
                                    rule: rule.main_rule.clone(),
                                    user_account: env.contract.address.to_string(),
                                    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                                    user_account_code_hash: Some(env.contract.code_hash.clone()),
                                    #[cfg(feature = "cosmwasm")]
                                    user_account_code_hash: None,
                                    rule_id: rule.id.unwrap(),
                                },
                            )?;
                        if let Some(auths) = gatekeeper_res.authorizations {
                            if !auths.authorizations.is_empty() {
                                return Ok(CanExecuteResponse {
                                    can_execute: CanExecute::No(
                                        CannotExecuteReason::GlobalBlocklist,
                                    ),
                                    reduce_spendlimit_msg: None,
                                });
                            }
                        };
                    }
                }
                _ => return Err(ContractError::Account(AccountError::InvalidGlobalRule {})),
            }
        }
        Ok(CanExecuteResponse {
            can_execute: CanExecute::Maybe(PendingReason::NotBlocklisted),
            reduce_spendlimit_msg: None,
        })
    }

    pub fn get_debt(&self, deps: Deps) -> StdResult<OutstandingDebtResponse> {
        let unwrapped_contract_addr = self
            .debtkeeper_contract_addr
            .clone()
            .ok_or_else(|| {
                ContractError::Std(StdError::generic_err("No known debt gatekeeper address"))
            })
            .unwrap();

        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let unwrapped_code_hash = self
            .debtkeeper_code_hash
            .clone()
            .ok_or_else(|| {
                ContractError::Std(StdError::generic_err("No known debt gatekeeper code hash"))
            })
            .unwrap();

        let debt_msg: DebtQueryMsg = OutstandingDebt {};

        let encoded_msg = to_binary(&debt_msg)?;

        deps.api.debug("Inter-contract query: \x1b[1;34mUser Account\x1b[0m querying \x1b[1;34mDebt Gatekeeper\x1b[0m");
        // println!("Encoded message: {:?}", encoded_msg);

        let query_response: Result<OutstandingDebtResponse, StdError> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: unwrapped_contract_addr,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                code_hash: unwrapped_code_hash,
                msg: encoded_msg,
            }));
        query_response
    }

    pub fn get_debt_repay_msg(
        &self,
        deps: Deps,
        env: Env,
        debt_response: OutstandingDebtResponse,
        repay_denom: String,
    ) -> Result<Option<CosmosMsg>, ContractError> {
        if debt_response.amount == Uint256::zero() {
            Ok(None)
        } else {
            // if multi-asset, for now we assume that first asset spend
            // is asset used to repay
            let unconverted_debt: SourcedCoins = SourcedCoins {
                coins: vec![CoinBalance {
                    amount: debt_response.amount,
                    denom: repay_denom,
                    spent_this_inheritance_period: None,
                    limit_remaining: Uint256::from(0u128),
                }],
                wrapped_sources: Sources { sources: vec![] },
            };
            let converted_debt = unconverted_debt.convert_to_base_or_target_asset(
                deps,
                self.asset_unifier_contract_addr.clone().unwrap(),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                self.asset_unifier_code_hash.clone().unwrap(),
                true,
                env.block.chain_id,
                None,
            )?;
            let repay_msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
                // unsafe unwrap, but we know this exists if debt is active
                to_address: self.fee_pay_wallet.clone().unwrap(),
                amount: vec![Coin {
                    denom: converted_debt.asset_unifier.denom.clone(),
                    amount: Uint128::try_from(converted_debt.asset_unifier.amount)
                        .map_err(ContractError::ConversionOverflow)?,
                }],
            });
            Ok(Some(repay_msg))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn query_gatekeeper<'a, T>(
        &self,
        deps: Deps,
        gatekeeper_contract_addr: Option<String>,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] gatekeeper_code_hash: String,
        query_msg: impl Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
        default_response: T,
        call_location: String,
        parent_call_location: Option<String>,
    ) -> Result<T, StdError>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        if let Some(contract_addr) = gatekeeper_contract_addr {
            deps.api.debug("Inter-contract query: \x1b[1;34mUser Account\x1b[0m querying \x1b[1;34mGatekeeper\x1b[0m");
            deps.api.debug(&format!(
                "encoded query message: {:?}",
                to_binary(&query_msg)?
            ));
            let res = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_addr.clone(),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                code_hash: gatekeeper_code_hash.clone(),
                msg: to_binary(&query_msg)?,
            }));
            if let Err(e) = res {
                Err(StdError::generic_err(format!(
                    "{}, called by {}, called by {:?}, {}, contract {}, code hash if known: {}, message {:?}",
                    macros::loc_string!(),
                    call_location,
                    parent_call_location,
                    e,
                    contract_addr,
                    {
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        {
                            gatekeeper_code_hash
                        }
                        #[cfg(feature = "cosmwasm")]
                        ""
                    },
                    to_binary(&query_msg)?
                )))
            } else {
                res
            }
        } else {
            Ok(default_response)
        }
    }

    pub fn set_debtkeeper_contract(&mut self, debtkeeper_addr: Option<String>) {
        self.debtkeeper_contract_addr = debtkeeper_addr.or(self.debtkeeper_contract_addr.clone());
    }

    pub fn set_user_state_contract(&mut self, user_state_addr: Option<String>) {
        self.user_state_contract_addr = user_state_addr.or(self.user_state_contract_addr.clone());
    }

    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    pub fn set_user_state_code_hash(&mut self, user_state_code_hash: Option<String>) {
        self.user_state_code_hash = user_state_code_hash.or(self.user_state_code_hash.clone());
    }
}

#[uniserde::uniserde]
pub struct UpdateDelayResponse {
    pub update_delay: u64,
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct GatekeeperContractsResponse {
    pub gatekeepers: Vec<String>,
    pub user_state_contract_addr: Option<String>,
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct GatekeeperContractsResponse {
    pub gatekeepers: Vec<(String, String)>,
    pub user_state_contract_addr: Option<String>,
    pub user_state_code_hash: Option<String>,
}

#[uniserde::uniserde]
pub struct PendingOwnerResponse {
    pub pending_owner: String,
}

#[uniserde::uniserde]
pub struct NextHashResponse {
    pub next_hash: String,
}

#[uniserde::uniserde]
pub struct SignersResponse {
    pub signers: Signers,
    pub evm_contract_address: Option<String>,
    pub evm_signing_address: Option<String>,
}

#[uniserde::uniserde]
pub struct CanExecuteResponse {
    pub can_execute: CanExecute,
    pub reduce_spendlimit_msg: Option<CosmosMsg>,
}

#[uniserde::uniserde]
pub struct GatekeeperCodeIds {
    pub spendlimit: Option<u64>,
    pub sessionkey: Option<u64>,
    pub message: Option<u64>,
    pub user_state: Option<u64>,
}
