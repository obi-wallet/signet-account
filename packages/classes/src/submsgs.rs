#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(
    from_binary,
    BankMsg,
    Binary,
    Coin,
    CosmosMsg,
    StdError,
    WasmMsg,
    Uint128
);
#[cfg(feature = "cosmwasm")]
use crate::legacy_cosmosmsg as LegacyMsg;
use common::universal_msg::UniversalMsg;
#[cfg(feature = "cosmwasm")]
use cw20::Cw20ExecuteMsg;

#[uniserde::uniserde]
pub struct PendingSubmsg {
    pub msg: UniversalMsg,
    pub contract_addr: Option<String>,
    pub binarymsg: Option<Binary>,
    pub funds: Vec<Coin>,
    pub ty: SubmsgType,
}

#[uniserde::uniserde]
pub struct PendingSubmsgGroup {
    msgs: Vec<PendingSubmsg>,
}

#[uniserde::uniserde]
pub enum SubmsgType {
    BankSend,
    BankBurn,
    ExecuteWasm(WasmMsgType),
    Unknown,
}

#[uniserde::uniserde]
pub enum WasmMsgType {
    Cw20Transfer,
    Cw20Burn,
    Cw20Send,
    Cw20IncreaseAllowance,
    Cw20DecreaseAllowance,
    Cw20TransferFrom,
    Cw20SendFrom,
    Cw20BurnFrom,
    Cw20Mint,
    Cw20UpdateMarketing,
    Cw20UploadLogo,
}

impl PendingSubmsg {
    pub fn add_funds(&mut self, funds: Vec<Coin>) {
        self.funds.extend(funds);
    }

    #[allow(clippy::collapsible_match)]
    pub fn process_and_get_msg_type(&mut self) -> SubmsgType {
        match &self.msg {
            // Restore support by handling these messages: cosmwasm-std can't be imported,
            // or compiles fail, but the secret-cosmwasm-std equivalent has code_hash.
            // Ideally we have .into() support for secret<>cosmwasm messages too.
            #[cfg(feature = "cosmwasm")]
            UniversalMsg::Legacy(cosmos_msg) => {
                // Restore support by handling these messages: cosmwasm-std can't be imported,
                // or compiles fail, but the secret-cosmwasm-std equivalent has code_hash.
                // Ideally we have .into() support for secret<>cosmwasm messages too.
                #[cfg(feature = "cosmwasm")]
                match cosmos_msg {
                    LegacyMsg::CosmosMsg::Wasm(LegacyMsg::WasmMsg::Execute {
                        contract_addr,
                        msg,
                        funds,
                    }) => {
                        self.contract_addr = Some(contract_addr.to_string());
                        self.binarymsg = Some(Binary(msg.clone().0));
                        #[cfg(feature = "cosmwasm")]
                        {
                            self.funds = funds.clone();
                        }
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        {
                            self.funds = funds
                                .clone()
                                .into_iter()
                                .map(|c| secret_cosmwasm_std::Coin {
                                    amount: secret_cosmwasm_std::Uint128::from(c.amount.u128()),
                                    denom: c.denom,
                                })
                                .collect::<Vec<secret_cosmwasm_std::Coin>>();
                        }
                        // note that parent message may have more funds attached
                        self.ty = self.process_execute_type();
                        self.ty.clone()
                    }
                    LegacyMsg::CosmosMsg::Bank(_) => {
                        self.contract_addr = None;
                        self.binarymsg = None;
                        self.funds = vec![];
                        // note that parent message may have more funds attached
                        self.ty = self.process_bank_type();
                        self.ty.clone()
                    }
                    _ => SubmsgType::Unknown,
                }
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                SubmsgType::Unknown
            }
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            UniversalMsg::Secret(secret_msg) => {
                #[allow(clippy::collapsible_match)]
                match secret_msg {
                    // SNIP-20 not supported yet. This is old cw20 code
                    #[cfg(feature = "cosmwasm")]
                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr,
                        msg,
                        funds,
                        code_hash,
                    }) => {
                        self.contract_addr = Some(contract_addr.to_string());
                        self.binarymsg = Some(msg.clone());
                        self.funds = funds.clone();
                        // note that parent message may have more funds attached
                        self.ty = self.process_execute_type();
                        self.ty.clone()
                    }
                    CosmosMsg::Bank(_) => {
                        self.contract_addr = None;
                        self.binarymsg = None;
                        self.funds = vec![];
                        // note that parent message may have more funds attached
                        self.ty = self.process_bank_type();
                        self.ty.clone()
                    }
                    _ => SubmsgType::Unknown,
                }
            }
            _ => SubmsgType::Unknown, // not supported and should be unreachable
        }
    }

    #[cfg(feature = "cosmwasm")]
    pub fn process_execute_type(&mut self) -> SubmsgType {
        let msg_de: Result<cw20::Cw20ExecuteMsg, StdError> = match &self.binarymsg {
            None => Err(StdError::GenericErr {
                msg: "Message does not exist as struct member".to_string(),
            }),
            Some(msg) => from_binary(msg),
        };
        match msg_de {
            Ok(msg_contents) => {
                // must be Transfer or Send if permissioned address
                match msg_contents {
                    Cw20ExecuteMsg::Transfer {
                        recipient: _,
                        amount,
                    } => {
                        if let Some(denom) = self.contract_addr.clone() {
                            // maybe this needs better handling
                            self.funds.push(Coin {
                                amount: Uint128::from(amount.u128()),
                                denom,
                            });
                            println!("pushed funds: {:?}", self.funds);
                        }
                        SubmsgType::ExecuteWasm(WasmMsgType::Cw20Transfer)
                    }
                    Cw20ExecuteMsg::Burn { amount } => {
                        if let Some(denom) = self.contract_addr.clone() {
                            // maybe this needs better handling
                            self.funds.push(Coin {
                                amount: Uint128::from(amount.u128()),
                                denom,
                            });
                        }
                        SubmsgType::ExecuteWasm(WasmMsgType::Cw20Burn)
                    }
                    Cw20ExecuteMsg::Send {
                        contract: _,
                        amount,
                        msg: _,
                    } => {
                        if let Some(denom) = self.contract_addr.clone() {
                            // maybe this needs better handling
                            self.funds.push(Coin {
                                amount: Uint128::from(amount.u128()),
                                denom,
                            });
                        }
                        SubmsgType::ExecuteWasm(WasmMsgType::Cw20Send)
                    }
                    Cw20ExecuteMsg::IncreaseAllowance {
                        spender: _,
                        amount,
                        expires: _,
                    } => {
                        if let Some(denom) = self.contract_addr.clone() {
                            // maybe this needs better handling
                            self.funds.push(Coin {
                                amount: Uint128::from(amount.u128()),
                                denom,
                            });
                        }
                        SubmsgType::ExecuteWasm(WasmMsgType::Cw20IncreaseAllowance)
                    }
                    Cw20ExecuteMsg::DecreaseAllowance {
                        spender: _,
                        amount: _,
                        expires: _,
                    } => SubmsgType::ExecuteWasm(WasmMsgType::Cw20DecreaseAllowance),
                    Cw20ExecuteMsg::TransferFrom {
                        owner: _,
                        recipient: _,
                        amount: _,
                    } => SubmsgType::ExecuteWasm(WasmMsgType::Cw20TransferFrom),
                    Cw20ExecuteMsg::SendFrom {
                        owner: _,
                        contract: _,
                        amount: _,
                        msg: _,
                    } => SubmsgType::ExecuteWasm(WasmMsgType::Cw20SendFrom),
                    Cw20ExecuteMsg::BurnFrom {
                        owner: _,
                        amount: _,
                    } => SubmsgType::ExecuteWasm(WasmMsgType::Cw20BurnFrom),
                    Cw20ExecuteMsg::Mint {
                        recipient: _,
                        amount: _,
                    } => SubmsgType::ExecuteWasm(WasmMsgType::Cw20Mint),
                    Cw20ExecuteMsg::UpdateMarketing {
                        project: _,
                        description: _,
                        marketing: _,
                    } => SubmsgType::ExecuteWasm(WasmMsgType::Cw20UpdateMarketing),
                    Cw20ExecuteMsg::UploadLogo(_) => {
                        SubmsgType::ExecuteWasm(WasmMsgType::Cw20UploadLogo)
                    }
                    Cw20ExecuteMsg::UpdateMinter { new_minter: _ } => todo!(),
                }
            }
            Err(_) => SubmsgType::Unknown,
        }
    }

    pub fn process_bank_type(&mut self) -> SubmsgType {
        match self.msg.clone() {
            #[cfg(feature = "cosmwasm")]
            UniversalMsg::Legacy(cosmos_msg) => match cosmos_msg {
                LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Send {
                    to_address: _,
                    amount,
                }) => {
                    #[cfg(feature = "cosmwasm")]
                    self.funds.extend(amount);
                    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                    self.funds.extend(
                        amount
                            .into_iter()
                            .map(|c| secret_cosmwasm_std::Coin {
                                amount: secret_cosmwasm_std::Uint128::from(c.amount.u128()),
                                denom: c.denom,
                            })
                            .collect::<Vec<secret_cosmwasm_std::Coin>>(),
                    );
                    SubmsgType::BankSend
                }
                LegacyMsg::CosmosMsg::Bank(LegacyMsg::BankMsg::Burn { amount }) => {
                    #[cfg(feature = "cosmwasm")]
                    self.funds.extend(amount);
                    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                    self.funds.extend(
                        amount
                            .into_iter()
                            .map(|c| secret_cosmwasm_std::Coin {
                                amount: secret_cosmwasm_std::Uint128::from(c.amount.u128()),
                                denom: c.denom,
                            })
                            .collect::<Vec<secret_cosmwasm_std::Coin>>(),
                    );
                    SubmsgType::BankBurn
                }
                _ => SubmsgType::Unknown,
            },
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            UniversalMsg::Secret(secret_msg) => match secret_msg {
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: _,
                    amount,
                }) => {
                    self.funds.extend(amount);
                    SubmsgType::BankSend
                }
                CosmosMsg::Bank(BankMsg::Burn { amount }) => {
                    self.funds.extend(amount);
                    SubmsgType::BankBurn
                }
                _ => SubmsgType::Unknown,
            },
            _ => SubmsgType::Unknown, // Not supported and should be unreachable
        }
    }
}
