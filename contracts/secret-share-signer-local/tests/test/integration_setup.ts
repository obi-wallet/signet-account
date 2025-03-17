import { SecretNetworkClient } from "secretjs";
import assert from "assert";
import dotenv from "dotenv";
import fs from "fs";
import { loadEnvVars } from "./helpers";
import { ContractDict } from "./contract_dict";
import {
  initializeAndUploadContracts,
  instantiateContract,
  makeFlexAccount,
  initializeClientWithEnv,
} from "./client";
import { runTestFunction } from "./tester";
import * as test_functions from "./test_functions";
dotenv.config();

let NO_INIT = false;
let ETH_ONLY = false;
let NO_ETH_SESSIONKEY = false;
let SIGNER_ONLY = false;

export type Coin = {
  denom: string;
  amount: string;
};

// process.argv contains the command-line arguments
// process.argv[0] is the node executable
// process.argv[1] is the path of the script being run
// So we start checking from process.argv[2]
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === "--no-init") {
    NO_INIT = true;
  }
  if (process.argv[i] === "--eth-only") {
    ETH_ONLY = true;
  }
  if (process.argv[i] === "--no-eth-sessionkey") {
    NO_ETH_SESSIONKEY = true;
  }
  if (process.argv[i] === "--signer-only") {
    SIGNER_ONLY = true;
  }
}

export class Payload {
  account_creator_address: string;
  owner_address: string;
  owner_public_key?: string;
  signer_address?: string;
  user_account_address?: string;
  user_entry_address?: string;
  flex_account?: string;
  flex_actor?: string;
  amount?: string;
  denom?: string;
  recipient?: string;
  arbitrary?: any;
  spendlimit?: Coin;

  constructor(
    account_creator_address: string,
    owner_address: string,
    owner_public_key: string,
    user_account_address?: string,
    user_entry_address?: string
  ) {
    this.account_creator_address = account_creator_address;
    this.owner_address = owner_address;
    this.user_entry_address = user_entry_address;
    this.user_account_address = user_account_address;
    this.owner_public_key = owner_public_key;
  }

  with_spendlimit(spendlimit: Coin): Payload {
    this.spendlimit = spendlimit;
    return this;
  }

  with_arbitrary(arbitrary: any): Payload {
    this.arbitrary = arbitrary;
    return this;
  }

  // signing client tries to give some authorization to `flex_account`
  with_admin_action(flex_account: string): Payload {
    this.flex_account = flex_account;
    return this;
  }

  // `flex_actor` tries the action
  as_flex_actor(flex_actor: string): Payload {
    this.flex_actor = flex_actor;
    return this;
  }

  with_spend(recipient?: string, amount?: string, denom?: string): Payload {
    this.recipient = recipient ?? this.owner_address;
    this.amount = amount ?? "1000";
    this.denom = denom ?? "uscrt";
    // if (this.denom == "uscrt") { console.warn("warning: spendlimit in USCRT"); }
    return this;
  }
}

export type InnerFunctionType = (
  clientSigning: SecretNetworkClient,
  contractDict: ContractDict,
  payload: Payload,
  testLog: string
) => Promise<string>;

process.on("unhandledRejection", (reason, promise) => {
  console.error("Unhandled Rejection at:", promise, "reason:", reason);
});

async function instantiateOrRetrieveContract(
  client: SecretNetworkClient,
  contractDict: ContractDict,
  contractName: string,
  initArgs: object,
  codeDetailsPath: string,
  chain_id: string,
  admin?: string
) {
  let contractAttributes =
    contractDict.getFlattenedAttributes(contractName, chain_id);
  let contractAddress = contractAttributes!.contractAddress;

  if (!NO_INIT) {
    try {
      const codeHash = await client.query.compute.codeHashByContractAddress({
        contract_address: String(contractAddress),
      });
      assert(
        codeHash == contractDict.getFlattenedAttributes(contractName)!.codeHash,
        "Code hash mismatch"
      );
    } catch {
      contractAddress = contractName === "account_creator" ? "" : undefined;
    }

    if (!contractAddress || contractAddress === "") {
      return await instantiateContract(
        client,
        contractDict,
        contractName,
        initArgs,
        codeDetailsPath,
        admin
      );
    } else {
      console.log(`${contractName} already instantiated`);
      return contractAddress;
    }
  } else {
    console.log(`Skipping ${contractName} instantiation with no_init`);
    return contractAddress;
  }
}

let testCounter = 1; // only used to console log test number

type TestFunctionType = () => Promise<InnerFunctionType>;
type TestFunctionWithArgs = {
  clientSigning: SecretNetworkClient;
  testFunction: TestFunctionType;
  contractDict: ContractDict;
  args: Payload;
  shouldFail: boolean;
  description: string;
};

const runTests = async (testFunctionWithArgs: TestFunctionWithArgs) => {
  let testLog =
    "\n" +
    "***" +
    "Running test #" +
    testCounter +
    ": " +
    testFunctionWithArgs.testFunction.name +
    "\n";
  testCounter++;
  const innerFunction = await testFunctionWithArgs.testFunction();
  try {
    await runTestFunction(
      innerFunction,
      testFunctionWithArgs.clientSigning,
      testFunctionWithArgs.contractDict,
      testFunctionWithArgs.args,
      testLog
    );
    if (testFunctionWithArgs.shouldFail) {
      throw new Error(
        "Test " + testFunctionWithArgs.description + " passed unexpectedly"
      );
    } else {
      console.log(
        "Test " + testFunctionWithArgs.description + " passed as expected"
      );
    }
  } catch (e) {
    if (testFunctionWithArgs.shouldFail) {
      console.log(testLog);
      console.log(
        "Test " + testFunctionWithArgs.description + " failed as expected"
      );
    } else {
      await runTestFunction(
        innerFunction,
        testFunctionWithArgs.clientSigning,
        testFunctionWithArgs.contractDict,
        testFunctionWithArgs.args,
        testLog
      );
    }
  }
};

const makeTestFunction = (
  clientSigning: SecretNetworkClient,
  testFunction: TestFunctionType,
  contractDict: ContractDict,
  args: Payload,
  shouldFail: boolean,
  description: string
) => {
  const testFunctionWithArgs: TestFunctionWithArgs = {
    clientSigning,
    testFunction,
    contractDict,
    args,
    shouldFail,
    description,
  };
  return testFunctionWithArgs;
};

export async function setupTestEnv(mnemonic: string, codeDetailsPath: string) {
  const ENDPOINT = process.env.ENDPOINT!;
  const CHAINID = process.env.CHAINID!;
  const ERC20 = process.env.ERC20!;
  const ETH_RECIPIENT = process.env.ETH_RECIPIENT!;

  // await getFromFaucet("secret1nvcdlkggvj2lzf9qxalcudhljdgjhyxuz6c0jy");
  const [basicClient, contractDict] = await initializeAndUploadContracts(
    NO_INIT,
    mnemonic,
    codeDetailsPath
  );

  assert(basicClient !== undefined, "Client not initialized");
  assert(contractDict !== undefined, "Contract dict not initialized");

  let fee_manager_address = await instantiateOrRetrieveContract(
    basicClient,
    contractDict,
    "fee_manager",
    {
      fee_divisors: ["5", 1000],
      fee_pay_addresses: ["5", "c1D4F3dcc31d86A66de90c3C5986f0666cc52ce4"],
    },
    codeDetailsPath,
    CHAINID,
    process.env.SIGNER_ADMIN!,
  );

  await instantiateOrRetrieveContract(
    basicClient,
    contractDict,
    "secret_share_signer",
    {
      fee_manager_address,
      fee_manager_code_hash: contractDict.getFlattenedAttributes("fee_manager", CHAINID)!.codeHash!,
    },
    codeDetailsPath,
    CHAINID,
    process.env.SIGNER_ADMIN!,
  );

  await instantiateOrRetrieveContract(
    basicClient,
    contractDict,
    "pair_registry",
    {
      legacy_owner: basicClient.address,
    },
    codeDetailsPath,
    CHAINID,
    process.env.SIGNER_ADMIN!,
  );

  await instantiateOrRetrieveContract(
    basicClient,
    contractDict,
    "asset_unifier",
    {
      default_asset_unifier: "uscrt",
      home_network: CHAINID,
      legacy_owner: basicClient.address,
      pair_contract_registry: contractDict.getFlattenedAttributes(
        "pair_registry",
        CHAINID
      )!.contractAddress,
      pair_contract_registry_code_hash: contractDict.getFlattenedAttributes(
        "pair_registry"
      )!.codeHash,
    },
    codeDetailsPath,
    CHAINID,
    process.env.SIGNER_ADMIN!,
  );

  await instantiateOrRetrieveContract(
    basicClient,
    contractDict,
    "eth_interpreter",
    {},
    codeDetailsPath,
    CHAINID,
    process.env.SIGNER_ADMIN!,
  );

  await instantiateOrRetrieveContract(
    basicClient,
    contractDict,
    "gatekeeper_message",
    {
      eth_interpreter_address: contractDict.getFlattenedAttributes(
        "eth_interpreter",
        CHAINID
      )!.contractAddress,
      eth_interpreter_code_hash: contractDict.getFlattenedAttributes(
        "eth_interpreter",
      )!.codeHash,
    },
    codeDetailsPath,
    CHAINID,
    process.env.SIGNER_ADMIN!,
  );

  const user_account_code_id = contractDict.getFlattenedAttributes(
    "user_account",
    CHAINID
  )!.codeId;
  assert(
    user_account_code_id !== undefined,
    "user_account_code_id is undefined"
  );

  const user_entry_code_id = contractDict.getFlattenedAttributes(
    "user_entry",
    CHAINID
  )!.codeId;
  assert(user_entry_code_id !== undefined, "user_entry_code_id is undefined");

  const premigrate_user_entry_code_id = contractDict.getFlattenedAttributes(
    "premigrate_user_entry",
    CHAINID
  )!.codeId;
  assert(premigrate_user_entry_code_id !== undefined, "premigrate_user_entry_code_id is undefined");

  let gatekeeper_spendlimit_contract_address =
    await instantiateOrRetrieveContract(
      basicClient,
      contractDict,
      "gatekeeper_spendlimit",
      {
        asset_unifier_contract: contractDict.getFlattenedAttributes(
          "asset_unifier",
          CHAINID
        )!.contractAddress,
        asset_unifier_code_hash: contractDict.getFlattenedAttributes(
          "asset_unifier"
        )!.codeHash,
      },
      codeDetailsPath,
      CHAINID,
      process.env.SIGNER_ADMIN!,
    );

  let account_creator_address: string =
    (await instantiateOrRetrieveContract(
      basicClient,
      contractDict,
      "account_creator",
      {
        owner: basicClient.address,
        config: {
          debt_repay_address: basicClient.address,
          fee_pay_address: basicClient.address,
          debtkeeper_code_id: contractDict.getFlattenedAttributes(
            "debtkeeper",
            CHAINID
          )!.codeId,
          debtkeeper_code_hash: contractDict.getFlattenedAttributes(
            "debtkeeper"
          )!.codeHash,
          asset_unifier_address: contractDict.getFlattenedAttributes(
            "asset_unifier",
            CHAINID
          )!.contractAddress,
          asset_unifier_code_hash: contractDict.getFlattenedAttributes(
            "asset_unifier"
          )!.codeHash,
          default_gatekeepers: [
            [
              contractDict.getFlattenedAttributes(
                "gatekeeper_message",
                CHAINID
              )!.codeId,
              contractDict.getFlattenedAttributes("gatekeeper_message", CHAINID)!
                .contractAddress,
              contractDict.getFlattenedAttributes("gatekeeper_message")!.codeHash
            ],
            [
              contractDict.getFlattenedAttributes(
                "gatekeeper_spendlimit",
                CHAINID
              )!.codeId,
              contractDict.getFlattenedAttributes(
                "gatekeeper_spendlimit",
                CHAINID
              )!.contractAddress,
              contractDict.getFlattenedAttributes(
                "gatekeeper_spendlimit"
              )!.codeHash
            ]
          ],
          user_account_code_id: contractDict.getFlattenedAttributes(
            "user_account",
            CHAINID
          )!.codeId,
          user_account_code_hash:
            contractDict.getFlattenedAttributes("user_account")!.codeHash,
          user_state_code_id: contractDict.getFlattenedAttributes(
            "user_state",
            CHAINID
          )!.codeId,
          user_state_code_hash:
            contractDict.getFlattenedAttributes("user_state")!.codeHash,
          user_entry_code_id: 
          CHAINID != "secretdev-1"
            ? contractDict.getFlattenedAttributes(
              "premigrate_user_entry",
              CHAINID
            )!.codeId
            : contractDict.getFlattenedAttributes(
              "user_entry",
              CHAINID
            )!.codeId,
          user_entry_code_hash:
          CHAINID != "secretdev-1"
            ? contractDict.getFlattenedAttributes(
              "premigrate_user_entry"
            )!.codeHash
            : contractDict.getFlattenedAttributes(
              "user_entry"
            )!.codeHash,  
        },
      },
      codeDetailsPath,
      CHAINID,
      process.env.SIGNER_ADMIN!,
    ))!;

  let user_account_address: string | undefined;
  // we could get from code_details, but this gets from chain by code id
  try {
    let user_account_infos = (
      await basicClient.query.compute.contractsByCodeId({
        code_id: user_account_code_id.toString(),
      })
    ).contract_infos;
    if (user_account_infos === undefined) {
      user_account_address = undefined;
    } else {
      user_account_address =
        user_account_infos[user_account_infos.length - 1].contract_address;
    }
  } catch {
    user_account_address = undefined;
  }
  let user_entry_address;
  try {
    let user_entry_infos = (
      await basicClient.query.compute.contractsByCodeId({
        code_id: user_entry_code_id.toString(),
      })
    ).contract_infos;
    if (user_entry_infos === undefined) {
      user_entry_address = undefined;
    } else {
      user_entry_address =
        user_entry_infos[user_entry_infos.length - 1].contract_address;
    }
  } catch {
    user_entry_address = undefined;
  }
  const payload = new Payload(
    account_creator_address!,
    basicClient.address,
    "040ea90e713bcb02581de510611857770cb91b64969582b0943e3e7a5550b84856baa906964dca107a4401d325bd571faeca4270d22390f799a9cfb79e7456e458",
    user_account_address,
    user_entry_address
  );

  return {
    basicClient,
    contractDict,
    payload,
  };
}


