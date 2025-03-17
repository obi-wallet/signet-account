import { SecretNetworkClient, MsgSendParams, MsgExecuteContractParams, TxOptions, TxResponse, Wallet } from "secretjs";
import fs from "fs";
import {
  findContractAddress,
  jsonToBase64,
  hexToByteArrayJson,
  assembleEthCallData,
} from "./helpers";
import { ContractDict } from "./contract_dict";
import { Payload, InnerFunctionType } from "./integration_setup";
import { assert } from "chai";
import { sleep } from "./secretjs_utils";
import { log } from "console";
import { Sha256 } from "@cosmjs/crypto";
import {
  serializeSignDoc,
  StdFee,
  StdSignDoc,
} from "@cosmjs/amino";
import * as secp256k1package from 'secp256k1';
import { LCDClient, MsgExecuteContract, MsgSend, Wallet as LegacyWallet, BlockTxBroadcastResult } from "@terra-money/terra.js";
import { send } from "process";

export async function log_msg_contents(serialized: string) {
  fs.writeFile("./logs/last_msg_contents.log", serialized, "utf-8", (error) => {
    if (error) {
      console.log(error);
    }
  });
  // console.log("Message contents logged to ./logs/last_msg_contents.log");
}

export async function create_new_user_account_from_factory(
  client: SecretNetworkClient | LCDClient,
  contractDict: ContractDict,
  payload: Payload,
  user_state_to_keep?: string,
  sender_address?: string,
): Promise<String> {
  const CHAINID = process.env.CHAINID;
  if (client instanceof SecretNetworkClient) {
    sender_address = client.address;
  }
  let msg = {
    sender: sender_address!,
    contract_address: payload.account_creator_address!,
    code_hash: contractDict.getFlattenedAttributes(
      "account_creator",
      CHAINID!
    )!.codeHash,
    msg: {
      new_account: {
        owner: sender_address,
        signers: {
          signers: [
            {
              address: sender_address,
              ty: "test1",
              pubkey_base_64: "fakepubkey",
            },
            {
              address: sender_address,
              ty: "test2",
              pubkey_base_64: "fakepubkey",
            },
          ],
        },
        update_delay: 0,
        user_state: user_state_to_keep
          ? user_state_to_keep
          : null,
        user_state_code_hash: user_state_to_keep
          ? contractDict.getFlattenedAttributes("user_state")!.codeHash!
          : null,
        next_hash_seed: "1a2b3c",
        fee_debt: 0
      },
    },
  };
  await log_msg_contents(JSON.stringify(msg));
  let res = await safeExecuteContract(
    client,
    msg,
    {
      gasLimit: 4000000,
    },
  );
  // write res to local log file, overwriting any existing content
  // create the file if it doesn't exist
  fs.writeFile(
    "./logs/create_new_user_account_from_factory.log",
    JSON.stringify(res, null, 2),
    "utf-8",
    (error) => {
      if (error) {
        console.log(error);
      }
    }
  );
  console.log("TX hash: " + res.transactionHash + "\n");
  // don't do this since if we get res later it's just tx without code
  // if(res.code === 0) { throw new Error("Transaction failed");
  if (res.arrayLog === undefined) {
    throw new Error("No arrayLog in response");
  }
  let user_account_codeId: string =
    contractDict
      .getFlattenedAttributes("user_account", CHAINID)!
      .codeId?.toString() || "";
  if (user_account_codeId === "") {
    throw new Error("User account code ID not found");
  }
  let user_account_address = findContractAddress(
    res.arrayLog,
    user_account_codeId
  );
  if (user_account_address === undefined) {
    throw new Error("User account address not found in response");
  }
  if (!/^secret[a-zA-Z0-9]{39}$/.test(user_account_address.trim())) {
    throw new Error(
      "User account address not read successfully, got " +
      user_account_address
    );
  }
  contractDict.contracts
    .find((contract) => contract.name === "user_account")!
    .add_instance(user_account_address, CHAINID!);
  payload.user_account_address = user_account_address;

  let user_entry_address: string;
  if (user_state_to_keep === undefined) {
    console.log("Adding new user entry to code_details");
    // premigrate-user-entry; we test migrate to user-entry immediately
    let user_entry_codeId: string =
      CHAINID != "secretdev-1"
        ? contractDict
          .getFlattenedAttributes("premigrate_user_entry", CHAINID)!
          .codeId?.toString() || ""
        : contractDict
        .getFlattenedAttributes("user_entry", CHAINID)!
        .codeId?.toString() || "";
    if (user_account_codeId === "") {
      throw new Error("User entry code ID not found");
    }
    user_entry_address = findContractAddress(
      res.arrayLog,
      user_entry_codeId
    )!;
    payload.user_entry_address = user_entry_address;
    return user_entry_address!;
  } else {
    return user_account_address!;
  }
}

export async function fund_account(
  client: SecretNetworkClient | LCDClient,
  from_address: string,
  account_address: string,
  amount: number,
  wallet?: Wallet | LegacyWallet,
) {
  let params: MsgSendParams = {
    to_address: account_address,
    from_address,
    amount: [
      {
        amount: String(amount),
        denom: "uscrt",
      },
    ],
  };
  log_msg_contents(JSON.stringify(params));
  let res = await safeSend(
    client,
    params,
    {
      gasLimit: 50000,
    },
    wallet!
  );
  // write res to local log file, overwriting any existing content
  // create the file if it doesn't exist
  fs.writeFile(
    "./logs/fund_user_account.log",
    JSON.stringify(res, null, 2) || "",
    "utf-8",
    (error) => {
      if (error) {
        console.log(error);
      }
    }
  );
  console.log("TX hash: " + res.transactionHash! + "\n");
  // don't do this since if we get res later it's just tx without code
  // if(res.code === 0) { throw new Error("Transaction failed");
}

export async function try_terra_mpc_spend(
  client: SecretNetworkClient,
  contractDict: ContractDict,
  payload: Payload,
  shouldFail: boolean,
  privKey: Uint8Array,
): Promise<boolean> {
  let bank_send_msg = {
    legacy: {
      bank: {
        send: {
          to_address: payload.recipient!,
          from_address: payload.user_entry_address!,
          amount: [
            {
              amount: payload.amount!,
              denom: payload.denom!,
            },
          ],
        },
      },
    },
  };
  const signDoc: StdSignDoc = {
    chain_id: "phoenix-1",
    account_number: "0",
    sequence: "0",
    fee: {
      amount: [{
        amount: "1000",
        denom: "uluna",
      }],
      gas: "1500000",
    },
    msgs: [{
      type: "/cosmos.bank.v1beta1.MsgSend",
      value: bank_send_msg
    }],
    memo: "",
  }
  const serialized = serializeSignDoc(signDoc);
  const hashToSign = new Sha256(serialized).digest();

  // request the signature
  // using sign bytes for now (which will be phased out as it doesn't verify)
  const secretShareSigner = contractDict.getFlattenedAttributes("secret_share_signer", process.env.CHAINID!)!;
  const signature = secp256k1package.ecdsaSign(Buffer.from(hashToSign), privKey);
  const signatureHex = Buffer.from(signature.signature).toString('hex');

  const signBytesQueryMsg = {
    sign_bytes: {
      user_entry_address: payload.user_entry_address!,
      user_entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
      bytes: Buffer.from(hashToSign).toString('hex'),
      bytes_signed_by_signers: [
        signatureHex
      ],
    }
  }

  const res = safeQuery(
    client,
    {
      contract_address: secretShareSigner.contractAddress,
      code_hash: secretShareSigner.codeHash,
      query: signBytesQueryMsg
    },
  );
  console.log(res);
  return false;
}

// test a spend, with optional parameters in payload
export async function try_spend(
  client: SecretNetworkClient | LCDClient,
  contractDict: ContractDict,
  payload: Payload,
  shouldFail: boolean,
  wallet?: LegacyWallet,
): Promise<boolean> {
  /*
  console.log(
    "Trying to send " +
    payload.amount +
    " " +
    payload.denom +
    " to " +
    payload.recipient +
    "\n"
  );
  */
  // console.log("Caller is " + client.address);
  if (payload.user_entry_address === undefined) {
    throw new Error("User entry address not provided");
  }
  if (payload.user_account_address === undefined) {
    throw new Error("User account address not provided");
  }
  const user_entry_address_string = payload.user_entry_address;
  const user_account_address_string = payload.user_account_address;
  let bank_send_msg = {
    secret: {
      bank: {
        send: {
          to_address: payload.recipient,
          from_address: payload.user_entry_address,
          amount: [
            {
              amount: payload.amount,
              denom: payload.denom,
            },
          ],
        },
      },
    },
  };
  await log_msg_contents(JSON.stringify(bank_send_msg));
  const bank_send_msg_encoded = await jsonToBase64(bank_send_msg);

  if (payload.recipient === undefined) {
    throw new Error("Recipient address not set");
  }
  // we may need to wait for balance to update
  await sleep(12000);
  let recipient_balance = await safeQuery(
    client,
    {
      address: payload.recipient,
      denom: payload.denom,
    },
    1,
    "bank"
  );
  // console.log("Recipient balance before: " + recipient_balance.balance?.amount + "\n");

  // first query a can_execute to see if the transaction will succeed and get the can(not) execute reason
  let address;
  if (client instanceof SecretNetworkClient) {
    address = client.address;
  } else {
    assert(client instanceof LCDClient);
    address = wallet?.key.accAddress;
  }
  const query_res = await safeQuery(
    client,
    {
      contract_address: user_account_address_string,
      code_hash:
        contractDict.getFlattenedAttributes("user_account")!.codeHash,
      query: {
        can_execute: {
          address,
          msg: bank_send_msg,
          funds: [],
        },
      },
    },
  );
  console.log(
    "\n" + "CanExecute query says: " + JSON.stringify(query_res.can_execute!)
  );

  let res;
  if (client instanceof SecretNetworkClient) {
    res = await client.tx.compute.executeContract(
      {
        sender: client.address,
        contract_address: user_entry_address_string,
        code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash,
        msg: {
          execute: {
            msg: bank_send_msg_encoded,
          },
        },
      },
      {
        gasLimit: 350000,
      }
    );
  } else {
    assert(client instanceof LCDClient);
    const tx = await wallet!.createAndSignTx({
      msgs: [
        new MsgExecuteContract(
          wallet!.key.accAddress,
          user_entry_address_string,
          {
            execute: {
              msg: bank_send_msg_encoded,
            },
          },
        )
      ]
    });
    res = await client.tx.broadcastBlock(tx);
  }
  fs.writeFile(
    "./logs/bank_send_as_owner.log",
    JSON.stringify(res, null, 2),
    "utf-8",
    (error) => {
      if (error) {
        console.log(error);
      }
    }
  );
  let txHash, rawLog;
  if (client instanceof SecretNetworkClient) {
    txHash = (res as TxResponse).transactionHash;
    rawLog = (res as TxResponse).rawLog;
  } else {
    assert(client instanceof LCDClient);
    txHash = (res as BlockTxBroadcastResult).txhash;
    rawLog = (res as BlockTxBroadcastResult).raw_log;
  }
  console.log("TX hash: " + txHash + "\n");
  // console.log("Full response: " + JSON.stringify(res.rawLog));
  if (rawLog.indexOf("failed to execute message") > -1) {
    return shouldFail;
  }
  // check that balance of recipient has increased
  // we may need to wait for balance to update
  await sleep(12000);
  let recipient_balance_after = await safeQuery(
    client,
    {
      address: payload.recipient,
      denom: payload.denom,
    },
    1,
    "bank"
  );
  console.log("balance check: " + recipient_balance.balance?.amount +
    " becomes " + recipient_balance_after.balance?.amount);
  const balance_difference =
    parseInt(recipient_balance_after.balance.amount!) -
    parseInt(recipient_balance.balance?.amount! || "0");
  if (balance_difference === 0 && shouldFail) {
    return true;
  }
  if (balance_difference === parseInt(payload.amount!) && !shouldFail) {
    return true;
  }
  return false;
}

// create a sessionkey. Uses arbitrary as sessionkey duration
export async function create_sessionkey(
  client: SecretNetworkClient | LCDClient,
  contractDict: ContractDict,
  payload: Payload,
  shouldFail: boolean,
  defineEthContracts?: string[],
  wallet?: LegacyWallet,
): Promise<boolean> {
  if (payload.arbitrary === undefined) {
    throw new Error("Sessionkey duration not provided");
  }

  const sessionkey_spendlimit =
  {
    new_rule: {
      actor: payload.flex_account,
      ty: "spendlimit",
      main_rule: {
        spendlimit: {
          address: payload.flex_account,
          cooldown: 0,
          inheritance_records: [],
          offset: 0,
          period_multiple: 1,
          period_type: "days",
          spend_limits: [
            {
              amount: payload.spendlimit?.amount ? payload.spendlimit.amount : "500000000",
              current_balance: "0",
              limit_remaining: payload.spendlimit?.amount ? payload.spendlimit.amount : "500000000",
              denom: payload.spendlimit?.denom ? payload.spendlimit.denom : "uscrt",
            },
          ],
          expiration: Math.floor(Date.now() / 1000) + payload.arbitrary,
        },
      }
    }
  };
  const sessionkey_allowlist_abstraction_rule = {
    new_rule: {
      actor: payload.flex_account,
      ty: "allowlist",
      main_rule: {
        allow: {
          actor: payload.flex_account,
          // one minute
          expiration: Math.floor(Date.now() / 1000) + payload.arbitrary,
          contract: defineEthContracts ? defineEthContracts : null
        },
      },
    },
  };
  log_msg_contents(
    JSON.stringify(sessionkey_allowlist_abstraction_rule),
  );

  // use the client passed in as `admin`, which is always owner for successful tests
  // but can be attempted by other addresses for failure tests
  let address;
  if (client instanceof SecretNetworkClient) {
    address = client.address;
  } else {
    assert(client instanceof LCDClient);
    assert(wallet);
    address = wallet.key.accAddress;
  }
  const res = await safeExecuteContract(
    client!,
    {
      sender: address,
      contract_address: payload.user_account_address!,
      code_hash:
        contractDict.getFlattenedAttributes("user_account")!.codeHash,
      msg: {
        add_abstraction_rule: sessionkey_allowlist_abstraction_rule,
      },
    },
    {
      gasLimit: 1000000,
    },
  );
  // write res to local log file, overwriting any existing content
  // create the file if it doesn't exist
  fs.writeFile(
    "./logs/create_sessionkey_allowlist.log",
    JSON.stringify(res, null, 2),
    "utf-8",
    (error) => {
      if (error) {
        console.log(error);
      }
    }
  );
  if (Object.keys(sessionkey_spendlimit).length !== 0) {
    const res = await safeExecuteContract(
      client!,
      {
        sender: address,
        contract_address: payload.user_account_address!,
        code_hash:
          contractDict.getFlattenedAttributes("user_account")!.codeHash,
        msg: {
          add_abstraction_rule: sessionkey_spendlimit,
        },
      },
      {
        gasLimit: 1000000,
      },
    );
    // write res to local log file, overwriting any existing content
    // create the file if it doesn't exist
    fs.writeFile(
      "./logs/create_sessionkey_spendlimit.log",
      JSON.stringify(res, null, 2),
      "utf-8",
      (error) => {
        if (error) {
          console.log(error);
        }
      }
    );
  }
  console.log("TX hash: " + res.transactionHash + "\n");
  // console.log("Full response: " + res.rawLog + "\n");
  return (res.code === 0) === !shouldFail;
}

// query abstraction rule len
export async function assert_abstraction_rule_len(
  client: SecretNetworkClient | LCDClient,
  contractDict: ContractDict,
  payload: Payload,
  minimum_rule_count: number,
): Promise<boolean> {
  const res: any = await safeQuery(
    client,
    {
      contract_address: payload.user_account_address!,
      code_hash:
        contractDict.getFlattenedAttributes("user_account")!.codeHash,
      query: { gatekeeper_contracts: {} },
    },
  );
  const rule_res: any = await safeQuery(
    client,
    {
      contract_address: res.user_state_contract_addr!,
      code_hash: contractDict.getFlattenedAttributes("user_state")!.codeHash,
      query: { abstraction_rules: { ty: [] } },
    },
  );
  console.log(
    "Expected " +
    minimum_rule_count +
    " abstraction rules, found " +
    rule_res.rules!.length
  );
  if (rule_res.rules!.length !== minimum_rule_count) {
    console.log(
      "Abstraction Rules found: " + JSON.stringify(rule_res.rules!, null, 2)
    );
  }
  return (rule_res.rules.length >= minimum_rule_count);
}

export async function check_eth_userop_spendlimit(
  client: SecretNetworkClient,
  contractDict: ContractDict,
  payload: Payload,
  multicall: boolean,
  shouldFail: boolean,
) {
  const hex_data = assembleEthCallData(
    payload.recipient!,
    payload.amount!,
    payload.denom!,
    multicall,
  );
  // console.log("hex data: " + hex_data);
  const ethUserOp = {
    sender: "12a2Fd1adA63FBCA7Cd9ec550098D48600D6dDc7",
    nonce: "1",
    init_code: [],
    call_data: hexToByteArrayJson(hex_data.substring(2)),
    call_gas_limit: "79118",
    verification_gas_limit: "146778",
    pre_verification_gas: "48172",
    max_fee_per_gas: "10327",
    max_priority_fee_per_gas: "429",
    paymaster_and_data: hexToByteArrayJson(
      "e93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064b564460000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003495a86706e134c9c162d4a020097db67f95673294f1d7a42608633046b13ab7130f5918760717b4a74c347c595d38fc2bdeaa7dc17429a9f6c1ac3f344266e91b"
    ),
    signature: [],
  };
  // console.log("EthUserOp: " + JSON.stringify(ethUserOp, null, 2))
  let query_msg = {
    user_op_tx_valid: {
      user_op: ethUserOp,
      chain_id: "5",
      user_entry_address: contractDict.getFlattenedAttributes("user_entry", process.env.CHAINID)!.contractAddress!,
      user_entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
      sender: client.address,
    }
  };
  log_msg_contents(JSON.stringify(query_msg));
  const query_res = await safeQuery(
    client,
    {
      contract_address: contractDict.getFlattenedAttributes("secret_share_signer", process.env.CHAINID!)!.contractAddress,
      code_hash:
        contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash,
      query: query_msg
    },
  );
  console.log("TX valid result: " + JSON.stringify(query_res, null, 2));
  return (query_res.valid === !shouldFail);
}

// checks if the userop will pass the signer's fee checks
export async function check_eth_userop_fees(
  client: SecretNetworkClient,
  contractDict: ContractDict,
  payload: Payload,
  multicall: boolean,
  shouldFail: boolean,
  feeOverride?: number,
  feePayAddressOveride?: string,
  tokenOverride?: string,
): Promise<boolean> {
  const hex_data = assembleEthCallData(
    payload.recipient!,
    payload.amount!,
    payload.denom!,
    multicall,
    feeOverride,
    feePayAddressOveride,
    tokenOverride,
  );
  // console.log("hex data: " + hex_data);
  const ethUserOp = {
    sender: "12a2Fd1adA63FBCA7Cd9ec550098D48600D6dDc7",
    nonce: "1",
    init_code: [],
    call_data: hexToByteArrayJson(hex_data.substring(2)),
    call_gas_limit: "79118",
    verification_gas_limit: "146778",
    pre_verification_gas: "48172",
    max_fee_per_gas: "10327",
    max_priority_fee_per_gas: "429",
    paymaster_and_data: hexToByteArrayJson(
      "e93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064b564460000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003495a86706e134c9c162d4a020097db67f95673294f1d7a42608633046b13ab7130f5918760717b4a74c347c595d38fc2bdeaa7dc17429a9f6c1ac3f344266e91b"
    ),
    signature: [],
  };
  // console.log("EthUserOp: " + JSON.stringify(ethUserOp, null, 2))
  let query_msg = {
    user_op_fees_valid: {
      user_op: ethUserOp,
      chain_id: "5",
      user_entry_address: contractDict.getFlattenedAttributes("user_entry", process.env.CHAINID!)!.contractAddress!,
      user_entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
    }
  };
  log_msg_contents(JSON.stringify(query_msg));
  const query_res = await safeQuery(
    client,
    {
      contract_address: contractDict.getFlattenedAttributes("secret_share_signer", process.env.CHAINID!)!.contractAddress,
      code_hash:
        contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash,
      query: query_msg
    },
  );
  console.log("Fees valid result: " + JSON.stringify(query_res, null, 2));
  return (query_res.valid === !shouldFail);
}

export async function safeExecuteContract(
  client: SecretNetworkClient | LCDClient,
  params: MsgExecuteContractParams<object>,
  options: TxOptions | undefined,
  wallet?: LegacyWallet,
): Promise<any> {
  await new Promise((r) => setTimeout(r, 3000));
  try {
    if (client instanceof SecretNetworkClient) {
      let res = await client.tx.compute.executeContract(params, {
        ...options,
        broadcastTimeoutMs: 60000,
      });
      return res;
    } else {
      assert(client instanceof LCDClient);
      assert(wallet);
      const tx = await wallet.createAndSignTx({
        msgs: [
          new MsgExecuteContract(
            wallet.key.accAddress,
            params.contract_address,
            params.msg,
          )
        ]
      });
      const res = await client.tx
        .broadcastBlock(tx);
    }
  } catch (error) {
    const message: string = (error as { message: string }).message;

    // Extract the transaction hash from the error message
    const match = message.match(/tx not found: ([A-F0-9]{64})/);
    if (match) {
      const txHash = match[1];
      console.log(`Waiting for transaction ${txHash}...`);
      // Wait for 6 seconds
      await new Promise((r) => setTimeout(r, 6000));
      const txDetails = await safeQuery(client, { txHash }, 1, "tx");
      console.log(`Transaction details: ${JSON.stringify(txDetails)}`);
      // You can decide what to return in this case
      return txDetails;
    } else {
      console.log(`Retrying transaction...`);
      // Retry the transaction
      return await safeExecuteContract(client, params, options);
    }
  }
}

export async function safeSend(
  client: SecretNetworkClient | LCDClient,
  params: MsgSendParams,
  options: TxOptions | undefined,
  wallet?: Wallet | LegacyWallet,
  counter?: number
): Promise<any> {
  await new Promise((r) => setTimeout(r, 3000));
  if (counter === undefined) {
    counter = 1;
  } else {
    counter++;
  }
  try {
    if (client instanceof SecretNetworkClient) {
      let res = await client.tx.bank.send(params, {
        ...options,
        broadcastTimeoutMs: 60000,
      });
      return res;
    } else if (client instanceof LCDClient) {
      const tx = await (wallet as LegacyWallet).createAndSignTx({
        msgs: [
          new MsgSend(
            (wallet as LegacyWallet).key.accAddress,
            params.to_address,
            params.amount[0].amount + params.amount[0].denom
          )
        ]
      });
      const res = await client.tx
        .broadcastBlock(tx);
      return res;
    }
  } catch (error) {
    console.log("Matched error...");
    const message: string = (error as { message: string }).message;
    
    // Extract the transaction hash from the error message
    const match = message.match(/tx not found: ([A-F0-9]{64})/);
    if (match) {
      const txHash = match[1];
      console.log(`Waiting for transaction ${txHash}...`);
      // Wait for 6 seconds
      await new Promise((r) => setTimeout(r, 6000));
      const txDetails = await safeQuery(client, { txHash }, 1, "tx");
      console.log(`Transaction details: ${JSON.stringify(txDetails)}`);
      // You can decide what to return in this case
      return txDetails;
    } else {
      if (counter < 3) {
        console.log(`Retrying transaction...`);
        // Retry the transaction
        return await safeSend(client, params, options, wallet, counter);
      } else {
        console.log(`Transaction failed after 3 retries`);
      }
    }
  }
}

export async function safeQuery(
  client: SecretNetworkClient | LCDClient,
  params: any,
  counter?: number,
  type?: string
): Promise<any> {
  await new Promise((r) => setTimeout(r, 3000));
  if (type === undefined) {
    type = "compute";
  }
  if (counter === undefined) {
    counter = 1;
  } else {
    counter++;
  }
  try {
    let querier;

    switch (type) {
      case "compute": {
        if (client instanceof SecretNetworkClient) {
          return await client.query.compute.queryContract(params);
        } else if (client instanceof LCDClient) {
          return await client.wasm.contractQuery(
            params.contract_address,
            params.query
          )
        }
      }
      case "bank": {
        if (client instanceof SecretNetworkClient) {
          return await client.query.bank.balance(params);
        } else if (client instanceof LCDClient) {
          return await client.bank.balance(
            params.address,
            params.denom
          );
        }
      }
      case "tx": {
        if (client instanceof SecretNetworkClient) {
          return await client.query.getTx(params.txHash);
        } else if (client instanceof LCDClient) {
          return await client.tx.txInfo(
            params.txHash,
          )
        }
      }
      default: throw new Error("Invalid query type");
    }
  } catch (error) {
    const message: string = (error as { message: string }).message;

    console.warn(`Error occurred during query: ${message}`);
    if (counter < 3) {
      await new Promise((r) => setTimeout(r, 6000));
      console.log(`Retrying query...`);
      return await safeQuery(client, params, counter);
    } else {
      console.log(`Query failed after 3 retries`);
      throw error;
    }
  }
}

export async function wait30(): Promise<InnerFunctionType> {
  return async (
    client: SecretNetworkClient,
    contractDict: ContractDict,
    payload: Payload,
    testLog: string
  ) => {
    // wait for 30 seconds
    await new Promise((r) => setTimeout(r, 30000));
    return testLog;
  };
}
