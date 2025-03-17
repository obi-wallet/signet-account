// test/example.test.ts
import { assert, expect, use } from 'chai';
import * as fs from "fs";
import { existsSync } from "fs";
import hdkey from 'hdkey';
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing"
import {
    Bip39,
    EnglishMnemonic,
    HdPath,
    Secp256k1,
    Slip10,
    Slip10Curve,
    Slip10RawIndex
} from "@cosmjs/crypto";
import { Secp256k1HdWallet } from "@cosmjs/amino";
import { Key, LCDClient, MnemonicKey, MsgExecuteContract, Wallet as LegacyWallet } from '@terra-money/terra.js';
import {signingOfflineStageDistributed} from "./signing_offline_stage_distributed";

import {
    BroadcastMode,
    MsgExecuteContractResponse, MsgInstantiateContractResponse, MsgStoreCodeResponse, SecretNetworkClient, TxSender, Wallet,
} from "secretjs";
import { config } from "dotenv";

import { Parameters } from "./mpc-ecdsa-wasm-types";
import pkg from 'elliptic';
import BN from 'bn.js';
import { keygen } from "./keygen";
// import { create_signers_and_pre_sign } from "./create_signers_and_pre_sign";
import { createUserOperationHash, EthUserOp, transformEthUserOp } from "./eth_user_op";
import { SignatureRecid, verify } from "./signature";
import { Account, encodeSecp256k1Pubkey, sleep, waitForBlocks } from "./secretjs_utils";
import { Payload, setupTestEnv } from './integration_setup';
import { assert_abstraction_rule_len, create_new_user_account_from_factory, create_sessionkey, fund_account, log_msg_contents, safeExecuteContract, check_eth_userop_fees, try_spend, check_eth_userop_spendlimit, try_terra_mpc_spend } from './test_functions';
import { findContractAddress } from './helpers';
import { ContractDict } from './contract_dict';
import { Coin } from 'secretjs/src/protobuf/cosmos/base/v1beta1/coin';
import { ethers, hexlify, keccak256, sha256 } from 'ethers';

import * as secp256k1package from 'secp256k1';
import { CANCELLED } from 'dns';

import {
    createSigners,
    keygenSimulated,
    keyRefreshSimulated,
    Signer,
    signingOfflineStageSimulated
} from "@mpc-sdk/mpc-bindings";

const keygenAndSignMode: "distributed" | "simulated" = "simulated";

// update as required
const PUBKEY1 = "An9YoJRlklu1UeUuw/luOdbEEYoE+4d5OCVA0uzOwxG0";
const PUBKEY2 = "AhNVRGTwUUsvs/M/Le6c2fmS8fL9qnrV61GYM1QlFDcn";
const PUBKEY3 = "An1s9kzbE7Kd3rzVkmXcd9TJiJez+d1OJNAN13a72vmr";
// These will be needed when *querying* the signer with an associated permission.
const TEST_USER_OP_HASH = "0x3237715bcf5c565a7f45ef46bd8af9b616d5bc44224354b3b3a19c9d259c56cf";

type AbstractionRules = {
    rules: Array<AbstractionRule>
}

type AbstractionRule = {
    sub_rules: any
}

const { ec } = pkg;

let NO_INIT = false;
let NO_NEW_ACCOUNT = false;
let ETH_ONLY = false;
let NO_ETH_SESSIONKEY = false;
let SIGNER_ONLY = false;

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
    if (process.argv[i] === "--no-new-account") {
        NO_NEW_ACCOUNT = true;
    }
}


const loadEnv = () => {
    if (existsSync(".env.local")) {
        config({ path: ".env.local" });
    } else {
        config({ path: ".env" });
    }
};
loadEnv();
// Load .env file based on environment (development or test)
const envFile = process.env.NODE_ENV === "test" ? ".env.local" : ".env.example";
config({ path: envFile });
const ENDPOINT = process.env.ENDPOINT!!;
const chainId = process.env.CHAIN_ID!!;
const secp256k1 = new ec("secp256k1");

const keygenParams: Parameters = {
    parties: 3, threshold: 1
};

let exampleEthUserOp: EthUserOp = {
    sender: '0x12a2Fd1adA63FBCA7Cd9ec550098D48600D6dDc7',
    nonce: '0x1',
    initCode: '0x',
    callData: '0x18dfb3c7000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f20000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f20000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d600000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044a9059cbb000000000000000000000000c1d4f3dcc31d86a66de90c3c5986f0666cc52ce400000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000',
    callGasLimit: '0x189fa',
    verificationGasLimit: '0x243e2',
    preVerificationGas: '0xdbd4',
    maxFeePerGas: '0x18',
    maxPriorityFeePerGas: '0x2',
    paymasterAndData: '0xe93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064d9b24a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000308fc6c774d25c4db5197364dd174facbbf72871dae44c86929379765f8bc6705063cc07a45d3351c97a7703ff60a904135c756deb56a4cde775369316a19d9e1b'
}

const accounts: Account[] = [];

let contract: string;
let basicClient: SecretNetworkClient;
let contractDict: ContractDict;
let payload: Payload;

function utf8ArrayToObject(data: Uint8Array): any {
    const decoder = new TextDecoder();
    return JSON.parse(decoder.decode(data));
}

function base64ToHex(base64: string) {
    return Array.from(atob(base64))
        .map(char => char.charCodeAt(0).toString(16).padStart(2, '0'))
        .join('');
}

function decompressPoint(compressedPointHex: string): string {
    // Decode the compressed point to get an elliptic curve point
    const point = secp256k1.curve.decodePoint(compressedPointHex, 'hex');

    // Retrieve the uncompressed x and y coordinates
    const x = point.getX().toString(16).padStart(64, '0');
    const y = point.getY().toString(16).padStart(64, '0');

    // Create the uncompressed hex string
    return x + y;
}

function signing_offline_stage(local_keys: any[], participatingPartyIds: number[], mode: String): any[] {
    const startTime = new Date(); // Start time
    let result;
    if (mode == "simulated") {
        result = signingOfflineStageSimulated(local_keys.filter((key, index) => participatingPartyIds.includes(key.i)));
    } else if (mode == "distributed") {
        result = signingOfflineStageDistributed(local_keys, participatingPartyIds);
    } else {
        throw new Error("Invalid signing mode: " + mode);
    }
    const endTime = new Date(); // End time
    const elapsedTime = endTime.getTime() - startTime.getTime(); // Calculate elapsed time
    console.log(`Elapsed time for signing_offline_stage: ${elapsedTime} ms`);
    return result;
}

async function query_either_client(
    contract_address: string,
    code_hash: string,
    query: object,
    CHAINID: string,
    client: SecretNetworkClient | LCDClient
): Promise<any> {
    let res;
    if (client instanceof SecretNetworkClient) {
        res = await client.query.compute.queryContract({
            contract_address,
            code_hash,
            query,
        });
    } else {
        assert(client instanceof LCDClient);
        res = await client.wasm.contractQuery(
            contract_address,
            query
        )
    }
    return res;
}

describe('Obi Tests', () => {
    const CHAINID = process.env.CHAINID!;
    const OWNER = 0;
    const FIRST_OWNER = 1;
    // We write contractDict to code_details.json for future reference
    describe('Instantiation and account creation', () => {
        const CHAINID = process.env.CHAINID!;
        it('upload and instantiate contracts', async function () {
            this.timeout(480000);
            let obj = await setupTestEnv(process.env.MNEMONIC!, './code_details.json');

            const mnemonics = [
                process.env.MNEMONIC,
                process.env.MNEMONIC2,
                process.env.MNEMONIC3,
                process.env.MNEMONIC4
            ];

            // Create clients for all the existing wallets in secretdev-1
            for (let i = 0; i < mnemonics.length; i++) {
                const mnemonic = mnemonics[i];
                if (mnemonic === undefined) {
                    throw new Error("Mnemonic not found");
                }
                {
                    const wallet = new Wallet(mnemonic);
                    accounts.push({
                        address: wallet.address, mnemonic: mnemonic, wallet: wallet, client: new SecretNetworkClient({
                            url: ENDPOINT, wallet: wallet, walletAddress: wallet.address, chainId,
                        }),
                    });
                }
            }

            basicClient = obj.basicClient;
            contractDict = obj.contractDict;
            payload = obj.payload;
            assert(basicClient.address !== undefined);
        });
        it('create a new user account', async () => {
            if (NO_NEW_ACCOUNT) {
                console.log("--no-new-account: Skipping create a new user account");
                return;
            }
            const msg = {
                sender: basicClient.address,
                contract_address: payload.account_creator_address,
                code_hash: contractDict.getFlattenedAttributes(
                    "account_creator",
                    CHAINID
                )!.codeHash,
                msg: {
                    new_account: {
                        owner: accounts[OWNER].address,
                        signers: {
                            signers: [
                                {
                                    address: accounts[OWNER].address,
                                    ty: "test1",
                                    pubkey_base_64: "An9YoJRlklu1UeUuw/luOdbEEYoE+4d5OCVA0uzOwxG0"
                                },
                                {
                                    address: accounts[OWNER].address,
                                    ty: "test2",
                                    pubkey_base_64: "An9YoJRlklu1UeUuw/luOdbEEYoE+4d5OCVA0uzOwxG0"
                                },
                            ],
                        },
                        update_delay: 0,
                        next_hash_seed: "1a2b3c",
                        fee_debt: 0
                    },
                },
            };
            await log_msg_contents(JSON.stringify(msg));
            let res = await safeExecuteContract(
                basicClient,
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
            // console.log("TX hash: " + res.transactionHash + "\n");
            let user_account_codeId: string =
                contractDict
                    .getFlattenedAttributes("user_account", CHAINID)!
                    .codeId?.toString() || "";
            let user_account_address = findContractAddress(
                res.arrayLog!,
                user_account_codeId!
            );
            if (!/^secret[a-zA-Z0-9]{39}$/.test(user_account_address!.trim())) {
                throw new Error(
                    "User account address not read successfully, got " +
                    user_account_address!
                );
            }
            contractDict.contracts
                .find((contract) => contract.name === "user_account")!
                .add_instance(user_account_address!, CHAINID);
            payload.user_account_address = user_account_address;

            let user_entry_code_id: string =
                CHAINID != "secretdev-1"
                    ? contractDict
                        .getFlattenedAttributes("premigrate_user_entry", CHAINID)!
                        .codeId?.toString() || ""
                    : contractDict
                        .getFlattenedAttributes("user_entry", CHAINID)!
                        .codeId?.toString() || "";
            let user_entry_address = findContractAddress(
                res.arrayLog!,
                user_entry_code_id!
            );
            if (!/^secret[a-zA-Z0-9]{39}$/.test(user_entry_address!.trim())) {
                throw new Error(
                    "User entry address not read successfully, got " +
                    user_entry_address!
                );
            }
            contractDict.contracts
                .find((contract) => contract.name === "user_entry")!
                .add_instance(user_entry_address!, CHAINID);
            contractDict.contracts
                .find((contract) => contract.name === "premigrate_user_entry")!
                .add_instance(user_entry_address!, CHAINID);
            payload.user_entry_address = user_entry_address;
            // write the updated file
            fs.writeFile(
                './code_details.json',
                JSON.stringify(contractDict.contracts, null, 2),
                "utf-8",
                (error) => {
                    if (error) {
                        console.error("Error writing file", error);
                    } else {
                        // console.log("Address saved to " + './code_details.json');
                    }
                }
            );
        });
        it('fund the new user account', async () => {
            const client = accounts[FIRST_OWNER].client;
            let address = "";
            let wallet = undefined;
            if (client instanceof LCDClient) {
                address = accounts[FIRST_OWNER].address;
                wallet = accounts[FIRST_OWNER].wallet;
            } else if (client instanceof SecretNetworkClient) {
                address = (accounts[FIRST_OWNER].client as SecretNetworkClient).address;
            }
            await fund_account(
                client,
                address,
                payload.user_entry_address!,
                500000,
                wallet!
            );
        });
        it('first_update_owner', async () => {
            if (NO_NEW_ACCOUNT) {
                console.log("--no-new-account: Skipping first_update_owner");
                return;
            }
            let contract_address;
            let code_hash;
            if (CHAINID != "secretdev-1") {
                contract_address = contractDict.getFlattenedAttributes("premigrate_user_entry", CHAINID)!.contractAddress!;
                code_hash = contractDict.getFlattenedAttributes("premigrate_user_entry")!.codeHash!;
            } else {
                contract_address = contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!;
                code_hash = contractDict.getFlattenedAttributes("user_entry")!.codeHash!;
            }
            const firstUpdateOwnerMsg = {
                sender: basicClient.address,
                contract_address,
                code_hash,
                msg: {
                    first_update_owner: {
                        first_owner: accounts[FIRST_OWNER].address,
                        // evm_contract_address: "test_address",
                        // evm_signing_address: "test_signer",
                        signers: {
                            signers: [{
                                ty: "firstowner",
                                address: accounts[FIRST_OWNER].address,
                                pubkey_base_64: PUBKEY2,
                            },
                            {
                                ty: "secondowner",
                                address: accounts[OWNER].address,
                                pubkey_base_64: PUBKEY1,
                            },
                            {
                                ty: "thirdowner",
                                address: accounts[2].address,
                                pubkey_base_64: PUBKEY3,
                            }]
                        }
                    }
                },
            };
            console.log("first owner message: " + JSON.stringify(firstUpdateOwnerMsg));

            await log_msg_contents(JSON.stringify(firstUpdateOwnerMsg));
            let res = await safeExecuteContract(
                basicClient,
                firstUpdateOwnerMsg,
                {
                    gasLimit: 400000,
                },
            );
            // write res to local log file, overwriting any existing content
            // create the file if it doesn't exist
            fs.writeFile(
                "./logs/first_update_owner.log",
                JSON.stringify(res, null, 2),
                "utf-8",
                (error) => {
                    if (error) {
                        console.log(error);
                    }
                }
            );
        })
    });

    describe('Account tests', async () => {
        before(async () => {
            //full sessionkey account
            await fund_account(
                accounts[OWNER].client,
                accounts[OWNER].address,
                accounts[3].address!,
                50000
            );
        });
        // doesn't run on localnet
        it('native wrapped migration (user entry only)', async () => {
            if (NO_NEW_ACCOUNT) {
                console.log("--no-new-account: Skipping native wrapped migration (user entry only)");
                return;
            }
            const pre_res = await (accounts[FIRST_OWNER].client as SecretNetworkClient).query.compute.contractInfo(
                {
                    contract_address: contractDict.getFlattenedAttributes("premigrate_user_entry", CHAINID)!.contractAddress!,
                }
            );
            if (CHAINID != "secretdev-1") {
                expect(pre_res.contract_info?.code_id!).to.equal("" + contractDict.getFlattenedAttributes("premigrate_user_entry", CHAINID)!.codeId);
            }
            if (CHAINID != "secretdev-1" && accounts[FIRST_OWNER].client instanceof SecretNetworkClient) {
                const tx = await accounts[FIRST_OWNER].client.tx.compute.executeContract({
                    sender: accounts[FIRST_OWNER].client.address,
                    contract_address: contractDict.getFlattenedAttributes("premigrate_user_entry", CHAINID)!.contractAddress!,
                    code_hash: contractDict.getFlattenedAttributes("premigrate_user_entry")!.codeHash!,
                    msg: {
                        wrapped_migrate: {
                            entry_code_id: contractDict.getFlattenedAttributes("user_entry", CHAINID)!.codeId!,
                            entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
                        }
                    },
                }, { gasLimit: 1_000_000, broadcastMode: BroadcastMode.Block });
                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);
                const res = await (accounts[FIRST_OWNER].client as SecretNetworkClient).query.compute.contractInfo(
                    {
                        contract_address: contractDict.getFlattenedAttributes("premigrate_user_entry", CHAINID)!.contractAddress!,
                    }
                );
                expect(res.contract_info?.code_id!).to.equal("" + contractDict.getFlattenedAttributes("user_entry", CHAINID)!.codeId);
            } else {
                console.log("Skipping native wrapped migration test for chain-id " + CHAINID);
            }
        });
        // (our owner spend test also funds the bad_spender, for fees)
        it('spend funds as owner', async () => {
            const asExpected = await try_spend(
                accounts[FIRST_OWNER].client,
                contractDict,
                payload.with_spend(accounts[OWNER].address, "100000"),
                false
            );
            assert(asExpected);
        });
        it('cannot spend funds as old owner', async () => {
            const asExpected = await try_spend(
                accounts[OWNER].client,
                contractDict,
                payload.with_spend(accounts[OWNER].address, "100000"),
                true
            );
            assert(asExpected);
        });
        it('cannot create session key as non-owner', async () => {
            const asExpected = await create_sessionkey(
                accounts[OWNER].client,
                contractDict,
                payload.with_admin_action(accounts[3].address)
                    .with_arbitrary(30),
                true,
            )
            assert(asExpected)
        });
        it('create session key as owner', async () => {
            const asExpected = await create_sessionkey(
                accounts[FIRST_OWNER].client,
                contractDict,
                payload.with_admin_action(accounts[3].address)
                    .with_arbitrary(30),
                false,
            )
            assert(asExpected)
        });
        it('query abstraction rules', async () => {
            assert(assert_abstraction_rule_len(
                accounts[FIRST_OWNER].client,
                contractDict,
                payload,
                2
            ))
        });
        it('execute a spend as sessionkey', async () => {
            const asExpected = await try_spend(
                accounts[3].client,
                contractDict,
                payload.with_spend(accounts[FIRST_OWNER].address, "50000"),
                false
            );
            assert(asExpected)
        });
        it('after 30 seconds, sessionkey has expired', async () => {
            console.log("Waiting 30 seconds...");
            await sleep(30000);
            const asExpected = await try_spend(
                accounts[3].client,
                contractDict,
                payload.with_spend(accounts[FIRST_OWNER].address, "50000"),
                true
            );
            assert(asExpected)
        });
        // Manual migration is enabled for chains which do not have native migration
        // (which user_entry allows via `WrappedMigrate`). This is a complex path
        // probably never needed again, so this test is not currently maintained.
        //
        // it('manual migration', async () => {
        //     // get current info
        //     type UserAccountAddressResponse = {
        //         user_account_address: string;
        //         user_account_code_hash: string;
        //     };
        //     let userAccountAddressBefore: UserAccountAddressResponse =
        //         await query_either_client(
        //             contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
        //             contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
        //             { user_account_address: {} },
        //             CHAINID,
        //             accounts[FIRST_OWNER].client
        //         );
        //     type GatekeeperContractsResponse = {
        //         gatekeepers: [[string, string]];
        //         user_state_contract_addr?: string;
        //         user_state_code_hash?: string;
        //     };
        //     const gatekeeperContractsBefore: GatekeeperContractsResponse =
        //         await query_either_client(
        //             userAccountAddressBefore.user_account_address!,
        //             userAccountAddressBefore.user_account_code_hash!,
        //             { gatekeeper_contracts: {} },
        //             CHAINID,
        //             accounts[FIRST_OWNER].client
        //         );
        //     assert(gatekeeperContractsBefore.user_state_contract_addr !== undefined, "user state contract address not retrieved");
        //     assert(gatekeeperContractsBefore.user_state_code_hash !== undefined, "user state code hash not retrieved");
        //     const abstractionRulesBefore: AbstractionRules =
        //         await query_either_client(
        //             gatekeeperContractsBefore.user_state_contract_addr!,
        //             gatekeeperContractsBefore.user_state_code_hash!,
        //             {
        //                 abstraction_rules: {
        //                     ty: ["allowlist", "spendlimit"]
        //                 }
        //             },
        //             CHAINID,
        //             accounts[FIRST_OWNER].client
        //         );
        //     console.log("Creating new account to attach...");
        //     // create updated account logic
        //     const userAccountAddress = await create_new_user_account_from_factory(
        //         accounts[FIRST_OWNER].client,
        //         contractDict,
        //         payload,
        //         gatekeeperContractsBefore.user_state_contract_addr!,
        //     );
        //     // update user account address
        //     console.log("Updating user account address...");
        //     if (accounts[FIRST_OWNER].client instanceof SecretNetworkClient) {
        //         const tx = await accounts[FIRST_OWNER].client.tx.compute.executeContract({
        //             sender: accounts[FIRST_OWNER].client.address,
        //             contract_address: contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
        //             code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
        //             msg: {
        //                 update_user_account_address: {
        //                     new_address: userAccountAddress,
        //                     new_code_hash: contractDict.getFlattenedAttributes("user_account")!.codeHash!,
        //                 }
        //             },
        //         }, { gasLimit: 1_000_000, broadcastMode: BroadcastMode.Block });
        //         if (tx.code !== 0) {
        //             console.log(tx.rawLog);
        //         }
        //         expect(tx.code).to.equal(0);
        //     } else {
        //         assert(accounts[FIRST_OWNER].wallet instanceof LegacyWallet);
        //         const tx = await accounts[FIRST_OWNER].wallet.createAndSignTx({
        //             msgs: [
        //                 new MsgExecuteContract(
        //                     accounts[FIRST_OWNER].wallet.key.accAddress,
        //                     contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
        //                     {
        //                         update_user_account_address: {
        //                             new_address: userAccountAddress,
        //                             new_code_hash: contractDict.getFlattenedAttributes("user_account")!.codeHash!,
        //                         }
        //                     },
        //                 )
        //             ]
        //         });
        //         const res = await accounts[FIRST_OWNER].client.tx.broadcastBlock(tx);
        //     }
        //     // check that the user account address has been updated
        //     const userAccountAddressAfter: UserAccountAddressResponse =
        //         await query_either_client(
        //             contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
        //             contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
        //             { user_account_address: {} },
        //             CHAINID,
        //             accounts[FIRST_OWNER].client
        //         );
        //     expect(userAccountAddressAfter).to.not.equal(userAccountAddressBefore);
        //     // update user account address in user state (passthrough)
        //     if (accounts[FIRST_OWNER].client instanceof SecretNetworkClient) {
        //         const user_state_tx = await accounts[FIRST_OWNER].client.tx.compute.executeContract({
        //             sender: accounts[FIRST_OWNER].client.address,
        //             contract_address: userAccountAddressBefore.user_account_address!,
        //             code_hash: userAccountAddressBefore.user_account_code_hash!,
        //             msg: {
        //                 update_user_state_account_address: {
        //                     new_user_account: userAccountAddressAfter.user_account_address,
        //                     new_user_account_code_hash: userAccountAddressAfter.user_account_code_hash,
        //                 }
        //             },
        //         }, { gasLimit: 1_000_000, broadcastMode: BroadcastMode.Block });
        //         assert(user_state_tx.code === 0, "failed to update user account: " + user_state_tx.rawLog);
        //     } else {
        //         assert(accounts[FIRST_OWNER].wallet instanceof LegacyWallet);
        //         const tx = await accounts[FIRST_OWNER].wallet.createAndSignTx({
        //             msgs: [
        //                 new MsgExecuteContract(
        //                     accounts[FIRST_OWNER].wallet.key.accAddress,
        //                     userAccountAddressBefore.user_account_address!,
        //                     {
        //                         update_user_state_account_address: {
        //                             new_user_account: userAccountAddressAfter.user_account_address,
        //                             new_user_account_code_hash: userAccountAddressAfter.user_account_code_hash,
        //                         }
        //                     },
        //                 )
        //             ]
        //         });
        //     }
        //     // check that the abstraction rules still remain
        //     // we have to query addresses again, otherwise we're just assuming
        //     // the same user state address
        //     const gatekeeperContractsAfter: GatekeeperContractsResponse =
        //         await query_either_client(
        //             userAccountAddressAfter.user_account_address,
        //             userAccountAddressAfter.user_account_code_hash,
        //             { gatekeeper_contracts: {} },
        //             CHAINID,
        //             accounts[FIRST_OWNER].client
        //         );

        //     const abstractionRulesAfter: AbstractionRules =
        //         await query_either_client(
        //             gatekeeperContractsAfter.user_state_contract_addr!,
        //             gatekeeperContractsAfter.user_state_code_hash!,
        //             {
        //                 abstraction_rules: {
        //                     ty: ["allowlist", "spendlimit"]
        //                 }
        //             },
        //             CHAINID,
        //             accounts[FIRST_OWNER].client
        //         )
        //     expect(abstractionRulesAfter.rules.length).to.equal(abstractionRulesBefore.rules.length);
        //     // update for future tests
        //     let contractDeployment = contractDict.contracts.find((contract) => contract.name === "user_account")!.deployments?.find((deployment) => deployment.chainId === CHAINID)!;
        //     contractDeployment.contractAddress = userAccountAddressAfter.user_account_address!;
        //     let userStateDeployment = contractDict.contracts.find((contract) => contract.name === "user_state")!.deployments?.find((deployment) => deployment.chainId === CHAINID)!;
        //     userStateDeployment.contractAddress = gatekeeperContractsAfter.user_state_contract_addr!;
        //     // write to file
        //     fs.writeFile(
        //         './code_details.json',
        //         JSON.stringify(contractDict.contracts, null, 2),
        //         "utf-8",
        //         (error) => {
        //             if (error) {
        //                 console.error("Error writing file", error);
        //             } else {
        //                 console.log("Addresses saved to " + './code_details.json');
        //             }
        //         }
        //     );
        //     payload.user_account_address = userAccountAddressAfter.user_account_address!;
        // });
        it('add a spend-limited sessionkey', async () => {
            if (NO_NEW_ACCOUNT) {
                console.log("--no-new-account: Skipping add a spend-limited sessionkey");
                return;
            }
            const asExpected = await create_sessionkey(
                accounts[FIRST_OWNER].client,
                contractDict,
                payload.with_admin_action(accounts[0].address)
                    .with_arbitrary(600)
                    .with_spendlimit({
                        denom: "uscrt",
                        amount: "5000"
                    }),
                false,
            )
            assert(asExpected)
        });
        it('spend-limited sessionkey cannot spend over limit', async () => {
            const success = await try_spend(
                accounts[0].client,
                contractDict,
                payload.with_spend(
                    accounts[FIRST_OWNER].address,
                    "5001",
                    "uscrt",
                ),
                true,
            )
            assert(success)
        });
        it('spend-limited sessionkey can spend limit', async () => {
            const success = await try_spend(
                accounts[0].client,
                contractDict,
                payload.with_spend(
                    accounts[FIRST_OWNER].address,
                    "5000",
                    "uscrt",
                ),
                false,
            )
            assert(success)
        });
        it('owner cannot sign userop without fee', async () => {
            {
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[FIRST_OWNER].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "4200",
                        process.env.ERC20!,
                    ),
                    false,
                    true,
                )
                assert(success)
            }
        });
        it('owner cannot sign userop with fee in wrong token', async () => {
            {
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[FIRST_OWNER].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "2000",
                        process.env.ERC20!,
                    ),
                    false,
                    true,
                    2,
                    "0xc1D4F3dcc31d86A66de90c3C5986f0666cc52ce4", //fee pay address
                    "0xf0F8FC7365C0c9F87189B6c8703e4719270A3318" //token override
                )
                assert(success)
            }
        });
        it('owner cannot sign userop with invalid fee', async () => {
            {
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[FIRST_OWNER].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "4200",
                        process.env.ERC20!,
                    ),
                    true,
                    true,
                    1,
                )
                assert(success)
            }
        });
        it('owner cannot sign userop with fee to wrong address', async () => {
            {
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[FIRST_OWNER].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "4000",
                        process.env.ERC20!,
                    ),
                    true,
                    true,
                    4,
                    "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                )
                assert(success)
            }
        });
        it('owner can sign userop with valid amount and fee', async () => {
            {
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[FIRST_OWNER].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "2000",
                        process.env.ERC20!,
                    ),
                    true,
                    false,
                    2
                )
                assert(success)
            }
        });
        it('create spend-limited sessionkey for eth asset', async () => {
            const payloadToSubmit = payload.with_admin_action(accounts[0].address)
                .with_arbitrary(600)
                .with_spendlimit({
                    denom: process.env.ERC20!,
                    amount: "2000"
                });
            const asExpected = await create_sessionkey(
                accounts[FIRST_OWNER].client,
                contractDict,
                payloadToSubmit,
                false,
                process.env.ERC20 ? [process.env.ERC20] : undefined
            )
            assert(asExpected)
        });
        it('spendlimit sessionkey cannot sign userop spending >limit', async () => {
            {
                assert(accounts[0].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_spendlimit(
                    accounts[0].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "2001",
                        process.env.ERC20!,
                    ),
                    true,
                    true
                );
                assert(success);
            }
        });
        it('spendlimit sessionkey cannot sign userop without fee', async () => {
            {
                assert(accounts[0].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[0].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "4200",
                        process.env.ERC20!,
                    ),
                    false,
                    true,
                )
                assert(success)
            }
        });
        it('spendlimit sessionkey cannot sign userop with invalid fee', async () => {
            {
                assert(accounts[0].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[0].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "4200",
                        process.env.ERC20!,
                    ),
                    true,
                    true,
                    1,
                )
                assert(success)
            }
        });
        it('spendlimit sessionkey cannot sign userop with fee to wrong address', async () => {
            {
                assert(accounts[0].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_fees(
                    accounts[0].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "4000",
                        process.env.ERC20!,
                    ),
                    true,
                    true,
                    4,
                    "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                )
                assert(success)
            }
        });
        // note: the *actual* call will fail as query (but not execute) on localsecret
        // and must use our own RPC nodes to ensure that doesn't happen.
        // We could also add a "simple spendlimit" for EVM sessionkeys.
        it('spendlimit sessionkey can sign userop spending ==limit', async () => {
            {
                assert(accounts[0].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_spendlimit(
                    accounts[0].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "2000",
                        process.env.ERC20!,
                    ),
                    true,
                    false,
                );
                assert(success);
            }
        });
        it('create a second spendlimit key', async () => {
            const asExpected = await create_sessionkey(
                accounts[FIRST_OWNER].client,
                contractDict,
                payload.with_admin_action(accounts[3].address)
                    .with_arbitrary(600)
                    .with_spendlimit({
                        denom: process.env.ERC20!,
                        amount: "1000000000000000000"
                    }),
                false,
                process.env.ERC20 ? [process.env.ERC20] : undefined
            )
            assert(asExpected)
        });
        // note: the *actual* call will fail as query (but not execute) on localsecret
        // and must use our own RPC nodes to ensure that doesn't happen.
        // We could also add a "simple spendlimit" for EVM sessionkeys.
        it('first spendlimit sessionkey can still sign userop', async () => {
            {
                assert(accounts[0].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_spendlimit(
                    accounts[0].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "2000",
                        process.env.ERC20!,
                    ),
                    true,
                    false,
                );
                assert(success);
            }
        });
        // note: the *actual* call will fail as query (but not execute) on localsecret
        // and must use our own RPC nodes to ensure that doesn't happen.
        // We could also add a "simple spendlimit" for EVM sessionkeys.
        it.skip('second spendlimit sessionkey cannot spend over limit', async () => {
            {
                assert(accounts[3].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_spendlimit(
                    accounts[3].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "1000000000000000001",
                        process.env.ERC20!,
                    ),
                    true,
                    true,
                );
                assert(success);
            }
        });
        // note: the *actual* call will fail as query (but not execute) on localsecret
        // and must use our own RPC nodes to ensure that doesn't happen.
        // We could also add a "simple spendlimit" for EVM sessionkeys.
        it('second spendlimit sessionkey can spend under limit', async () => {
            {
                assert(accounts[3].client instanceof SecretNetworkClient);
                const success = await check_eth_userop_spendlimit(
                    accounts[3].client,
                    contractDict,
                    payload.with_spend(
                        "0x5Ff137D4b0FDCD49DcA30c7CF57E578A026d2789",
                        "1000000000000000000",
                        process.env.ERC20!,
                    ),
                    true,
                    false,
                );
                assert(success);
            }
        });
    });

    describe('secret-share-signer tests', () => {
        {
            const CHAINID = process.env.CHAINID;
            let data: string;
            let USER_ENTRY_ADDRESS: string;
            let USER_ENTRY_CODE_HASH: string;
            let userop_signature_by_pubkey_hex: string;
            let userop_signature_by_spendlimit_pubkey_hex: string;
            let bad_userop_signature_by_pubkey_hex: string;

            // Prevent MP-ECDSA from flooding the console
            let originalConsoleInfo: typeof console.info;

            before(function () {
                originalConsoleInfo = console.info;
                console.info = function (...args: any[]) { };
            });

            after(function () {
                console.info = originalConsoleInfo;  // Restore original console.info after tests
            });

            before(async () => {
                assert(accounts[FIRST_OWNER].wallet instanceof Wallet);
                assert(accounts[3].wallet instanceof Wallet);
                assert(accounts[0].wallet instanceof Wallet);
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                assert(accounts[3].client instanceof SecretNetworkClient);
                assert(accounts[0].client instanceof SecretNetworkClient);
                USER_ENTRY_ADDRESS = contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!;
                USER_ENTRY_CODE_HASH = contractDict.getFlattenedAttributes("user_entry")!.codeHash!;
                assert(USER_ENTRY_CODE_HASH !== undefined, "User entry code hash not found");
                contract = contractDict.getFlattenedAttributes("secret_share_signer", CHAINID)!.contractAddress!;
                let userop_signature = secp256k1.sign(
                    Buffer.from(TEST_USER_OP_HASH.slice(2), 'hex'),
                    Buffer.from(accounts[FIRST_OWNER].wallet.privateKey),
                );
                userop_signature_by_pubkey_hex = userop_signature.r.toString('hex').padStart(64, '0') + userop_signature.s.toString('hex').padStart(64, '0');

                let spendlimit_userop_signature = secp256k1.sign(
                    Buffer.from(TEST_USER_OP_HASH.slice(2), 'hex'),
                    Buffer.from(accounts[3].wallet.privateKey),
                );
                userop_signature_by_spendlimit_pubkey_hex = spendlimit_userop_signature.r.toString('hex').padStart(64, '0') + spendlimit_userop_signature.s.toString('hex').padStart(64, '0');

                let bad_userop_signature = secp256k1.sign(
                    Buffer.from(TEST_USER_OP_HASH.slice(2), 'hex'),
                    Buffer.from(accounts[0].wallet.privateKey),
                );
                bad_userop_signature_by_pubkey_hex = bad_userop_signature.r.toString('hex').padStart(64, '0') + bad_userop_signature.s.toString('hex').padStart(64, '0');

                const mnemonics = [
                    process.env.MNEMONIC,
                    process.env.MNEMONIC2,
                    process.env.MNEMONIC3,
                    process.env.MNEMONIC4
                ];

                await waitForBlocks("secretdev-1", ENDPOINT);
            });
            it('(execute & query) cannot sign over spendlimit as spendlimit wallet', async () => {
                USER_ENTRY_ADDRESS = contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!;
                USER_ENTRY_CODE_HASH = contractDict.getFlattenedAttributes("user_entry")!.codeHash!;
                const partiesKeyShares = keygen(keygenParams);
                const participatingPartyIds = [1, 2];
                const signers = signing_offline_stage(partiesKeyShares, participatingPartyIds, keygenAndSignMode);
                // const signers = create_signers_and_pre_sign(partiesKeyShares.map((keyshare) => keyshare.localKey), participatingPartyIds);
                const contract_signer = signers[0];
                const contract_signers_completed_offline_stage = contract_signer.completedOfflineStage();
                //console.log(JSON.stringify(contract_signers_completed_offline_stage));

                const compressedR = contract_signers_completed_offline_stage.R.point;
                const decompressedR = decompressPoint(compressedR);
                const compressedPubKey = contract_signers_completed_offline_stage.local_key.y_sum_s.point;
                const decompressedPubKey = decompressPoint(compressedPubKey);

                // store key share in contract
                const { client: secretjs } = accounts[FIRST_OWNER];
                assert(secretjs instanceof SecretNetworkClient);
                let setShareMsg = {
                    set_shares: {
                        participants_to_completed_offline_stages: [{
                            participants: participatingPartyIds,
                            completed_offline_stage: {
                                k_i: contract_signers_completed_offline_stage.sign_keys.k_i.scalar,
                                R: decompressedR,
                                sigma_i: contract_signers_completed_offline_stage.sigma_i.scalar,
                                pubkey: decompressedPubKey,
                                user_entry_code_hash: USER_ENTRY_CODE_HASH,
                            }
                        }],
                        user_entry_address: USER_ENTRY_ADDRESS,
                    }
                };
                console.log(JSON.stringify(setShareMsg))
                let tx = await secretjs.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: secretjs.address, contract_address: contract, msg: setShareMsg,
                }, { gasLimit: 1_000_000, broadcastMode: BroadcastMode.Block });
                console.log("test 1 tx response: " + JSON.stringify(tx));
                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);

                console.log("Set key gas:", tx.gasUsed);

                // compute the Ethereum UserOp (ERC4337) hash
                let hash = createUserOperationHash(exampleEthUserOp, '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789', '5')
                //console.log(hash);
                expect(hash).to.equal(TEST_USER_OP_HASH);
                const hashByteArray = new Uint8Array(Buffer.from(hash.slice(2), 'hex'));
                // sign the eth signature with all but one of the signers locally
                var partialSignatures: any[] = [];
                for (let signersIndex = 1; signersIndex < signers.length; signersIndex++) {
                    partialSignatures.push(signers[signersIndex].partial(hashByteArray));
                    //console.log(JSON.stringify(partialSignatures[partialSignatures.length - 1]))
                }
                let aggregatedPartialSignature = signers[1].add(partialSignatures.slice(1)).scalar;
                let execute_sign_msg = {
                    sign: {
                        participants: participatingPartyIds,
                        user_entry_address: USER_ENTRY_ADDRESS,
                        user_entry_code_hash: USER_ENTRY_CODE_HASH,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: [aggregatedPartialSignature],
                    }
                }
                console.log("execute_sign_msg:\n" + JSON.stringify(execute_sign_msg))

                // By executing the contract with the sign message have the contract complete the signature with the last
                // remaining partial signature and return the full signature
                // Here, we try with spendlimit wallet since this is fail test
                assert(accounts[3].client instanceof SecretNetworkClient);
                assert(accounts[3].wallet instanceof Wallet);
                tx = await accounts[3].client.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: accounts[3].client.address, contract_address: contract, msg: execute_sign_msg
                }, {
                    gasLimit: 2_000_000
                });

                expect(tx.code).to.not.equal(0);

                // Querying also fails (note pubkey used is not owner's pubkey)
                let query_sign_msg = {
                    sign_userop: {
                        participants: participatingPartyIds,
                        user_entry_address: contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
                        user_entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: [aggregatedPartialSignature],
                        userop_signed_by_signers: [userop_signature_by_spendlimit_pubkey_hex],
                    }
                }
                try {
                    await secretjs.query.compute.queryContract({
                        code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                        contract_address: contract, query: query_sign_msg
                    });
                    // heh.
                    expect(true).to.be.false;
                }
                catch {
                    // expected
                }
            });
            it('query sign bytes as owner', async () => {
                USER_ENTRY_ADDRESS = contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!;
                USER_ENTRY_CODE_HASH = contractDict.getFlattenedAttributes("user_entry")!.codeHash!;
                
                console.log("keygen...");
                const partiesKeyShares = keygen(keygenParams);
                const participatingPartyIds = [1, 2];
                console.log("create_signers_and_pre_sign...");
                const signers = signing_offline_stage(partiesKeyShares, participatingPartyIds, keygenAndSignMode);
                // const signers = create_signers_and_pre_sign(partiesKeyShares.map((keyshare) => keyshare.localKey), participatingPartyIds);
                const contract_signer = signers[0];
                const contract_signers_completed_offline_stage = contract_signer.completedOfflineStage();
                //console.log(JSON.stringify(contract_signers_completed_offline_stage));

                const compressedR = contract_signers_completed_offline_stage.R.point;
                const decompressedR = decompressPoint(compressedR);
                const compressedPubKey = contract_signers_completed_offline_stage.local_key.y_sum_s.point;
                const decompressedPubKey = decompressPoint(compressedPubKey);
                console.log("store key shares...");
                // store key share in contract
                const { client: secretjs } = accounts[FIRST_OWNER];
                assert(secretjs instanceof SecretNetworkClient);
                let setShareMsg = {
                    set_shares: {
                        participants_to_completed_offline_stages: [{
                            participants: participatingPartyIds,
                            completed_offline_stage: {
                                k_i: contract_signers_completed_offline_stage.sign_keys.k_i.scalar,
                                R: decompressedR,
                                sigma_i: contract_signers_completed_offline_stage.sigma_i.scalar,
                                pubkey: decompressedPubKey,
                                user_entry_code_hash: USER_ENTRY_CODE_HASH,
                            }
                        }],
                        user_entry_address: USER_ENTRY_ADDRESS,
                    }
                };
                console.log(JSON.stringify(setShareMsg))
                let tx = await secretjs.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: secretjs.address, contract_address: contract, msg: setShareMsg,
                }, { gasLimit: 1_000_000, broadcastMode: BroadcastMode.Block });
                console.log("test 1 tx response: " + JSON.stringify(tx));
                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);

                console.log("Set key gas:", tx.gasUsed);

                const bytes = "0f6dbe57888cd614439bee3a4b11d13f5784b149e0a7ce91735a77b40475c48d";
                // sign the eth signature with all but one of the signers locally
                var partialSignatures: any[] = [];
                for (let signersIndex = 1; signersIndex < signers.length; signersIndex++) {
                    partialSignatures.push(signers[signersIndex].partial(Buffer.from(bytes, "hex")));
                    //console.log(JSON.stringify(partialSignatures[partialSignatures.length - 1]))
                }
                let aggregatedPartialSignature = signers[1].add(partialSignatures.slice(1)).scalar;
                
                const signatures = 
                [
                    secp256k1.sign(new BN(Buffer.from(bytes, "hex")), Buffer.from((accounts[1].wallet as Wallet).privateKey)),
                    secp256k1.sign(new BN(Buffer.from(bytes, "hex")), Buffer.from((accounts[2].wallet as Wallet).privateKey)),
                ];
                let bytes_signed_by_signers: string[] = [];
                signatures.forEach((sig) => {
                    bytes_signed_by_signers.push(sig.r.toString('hex').padStart(64, '0') + sig.s.toString('hex').padStart(64, '0'));
                });

                let query_sign_bytes_msg = {
                    sign_bytes: {
                        participants: participatingPartyIds,
                        user_entry_address: USER_ENTRY_ADDRESS,
                        user_entry_code_hash: USER_ENTRY_CODE_HASH,
                        other_partial_sigs: [aggregatedPartialSignature],
                        bytes,
                        bytes_signed_by_signers,
                        prepend: false,
                        is_already_hashed: true
                    }
                }
                console.log("Query sign bytes msg: " + JSON.stringify(query_sign_bytes_msg));
                let res;
                try {
                    res = await secretjs.query.compute.queryContract({
                        code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                        contract_address: contract, query: query_sign_bytes_msg
                    });
                    // heh.
                    expect(true).to.be.false;
                }
                catch {
                    // expected
                }
                console.log("Query sign bytes result: " + JSON.stringify(res));
                expect(res).to.not.be.undefined;
            });
            // need to update as user op hash (to be signed by signers) has changed
            it.skip('(execute & query) sign w/ aggregated single partial sig', async () => {
                USER_ENTRY_ADDRESS = contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!;
                USER_ENTRY_CODE_HASH = contractDict.getFlattenedAttributes("user_entry")!.codeHash!;
                const partiesKeyShares = keygen(keygenParams);
                const participatingPartyIds = [1, 2];
                const signers = signing_offline_stage(partiesKeyShares, participatingPartyIds, keygenAndSignMode);
                // const signers = create_signers_and_pre_sign(partiesKeyShares.map((keyshare) => keyshare.localKey), participatingPartyIds);
                const contract_signer = signers[0];
                const contract_signers_completed_offline_stage = contract_signer.completedOfflineStage();
                //console.log(JSON.stringify(contract_signers_completed_offline_stage));

                const compressedR = contract_signers_completed_offline_stage.R.point;
                const decompressedR = decompressPoint(compressedR);
                const compressedPubKey = contract_signers_completed_offline_stage.local_key.y_sum_s.point;
                const decompressedPubKey = decompressPoint(compressedPubKey);

                // store key share in contract
                const { client: secretjs } = accounts[FIRST_OWNER];
                let setShareMsg = {
                    set_shares: {
                        participants_to_completed_offline_stages: [{
                            participants: participatingPartyIds,
                            completed_offline_stage: {
                                k_i: contract_signers_completed_offline_stage.sign_keys.k_i.scalar,
                                R: decompressedR,
                                sigma_i: contract_signers_completed_offline_stage.sigma_i.scalar,
                                pubkey: decompressedPubKey,
                                user_entry_code_hash: USER_ENTRY_CODE_HASH,
                            }
                        }],
                        user_entry_address: USER_ENTRY_ADDRESS,
                    }
                };
                console.log(JSON.stringify(setShareMsg))
                assert(secretjs instanceof SecretNetworkClient);
                let tx = await secretjs.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: secretjs.address, contract_address: contract, msg: setShareMsg,
                }, { gasLimit: 1_000_000, broadcastMode: BroadcastMode.Block });
                console.log("test 1 tx response: " + JSON.stringify(tx));
                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);

                console.log("Set key gas:", tx.gasUsed);

                // compute the Ethereum UserOp (ERC4337) hash
                let hash = createUserOperationHash(exampleEthUserOp, '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789', '5')
                //console.log(hash);
                expect(hash).to.equal(TEST_USER_OP_HASH);
                const hashByteArray = new Uint8Array(Buffer.from(hash.slice(2), 'hex'));
                // sign the eth signature with all but one of the signers locally
                var partialSignatures: any[] = [];
                for (let signersIndex = 1; signersIndex < signers.length; signersIndex++) {
                    partialSignatures.push(signers[signersIndex].partial(hashByteArray));
                    //console.log(JSON.stringify(partialSignatures[partialSignatures.length - 1]))
                }
                let aggregatedPartialSignature = signers[1].add(partialSignatures.slice(1)).scalar;
                let execute_sign_msg = {
                    sign: {
                        participants: participatingPartyIds,
                        user_entry_address: USER_ENTRY_ADDRESS,
                        user_entry_code_hash: USER_ENTRY_CODE_HASH,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: [aggregatedPartialSignature],
                    }
                }
                console.log("execute_sign_msg:\n" + JSON.stringify(execute_sign_msg))

                // By executing the contract with the sign message have the contract complete the signature with the last
                // remaining partial signature and return the full signature
                tx = await secretjs.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: secretjs.address, contract_address: contract, msg: execute_sign_msg
                }, {
                    gasLimit: 2_000_000
                });

                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);
                console.log("Sign gas:", tx.gasUsed);
                let sig: SignatureRecid = utf8ArrayToObject(MsgExecuteContractResponse.decode(tx.data[0]).data)
                console.log("Final signature from execute contract:\n" + JSON.stringify(sig))
                const publicKey = secp256k1.keyFromPublic(compressedPubKey, 'hex');

                const y = publicKey.getPublic();
                expect(verify(secp256k1, sig, y, new BN(hashByteArray, 16))).to.equal(true);

                // Now by querying the contract with the sign message have the contract complete the signature with the last
                // remaining partial signature and return the full signature
                assert(accounts[FIRST_OWNER].wallet instanceof Wallet);
                let query_sign_msg = {
                    sign_userop: {
                        participants: participatingPartyIds,
                        user_entry_address: contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
                        user_entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: [aggregatedPartialSignature],
                        userop_signed_by_signers: [userop_signature_by_pubkey_hex],
                    }
                }
                console.log("query_sign_msg:\n" + JSON.stringify(query_sign_msg))
                const querySig: SignatureRecid = await secretjs.query.compute.queryContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    contract_address: contract, query: query_sign_msg
                });
                console.log("Final signature from query contract:\n" + JSON.stringify(querySig))
                expect(querySig).to.deep.equal(sig);
            });
            // need to update as user op hash (to be signed by signers) has changed
            it.skip('(execute & query) sign w/ multiple partial sigs', async () => {
                const partiesKeyShares = keygen(keygenParams);
                const participatingPartyIds = [1, 2];
                const signers = signing_offline_stage(partiesKeyShares, participatingPartyIds, keygenAndSignMode);
                // const signers = create_signers_and_pre_sign(partiesKeyShares.map((keyshare) => keyshare.localKey), participatingPartyIds);

                const contract_signer = signers[0];
                const contract_signers_completed_offline_stage = contract_signer.completedOfflineStage();
                //console.log(JSON.stringify(contract_signers_completed_offline_stage));

                const compressedR = contract_signers_completed_offline_stage.R.point;
                const decompressedR = decompressPoint(compressedR);
                const compressedPubKey = contract_signers_completed_offline_stage.local_key.y_sum_s.point;
                const decompressedPubKey = decompressPoint(compressedPubKey);

                // store key share in contract
                const { client: secretjs } = accounts[FIRST_OWNER];
                let setShareMsg = {
                    set_shares: {
                        participants_to_completed_offline_stages: [{
                            participants: participatingPartyIds, completed_offline_stage: {
                                k_i: contract_signers_completed_offline_stage.sign_keys.k_i.scalar,
                                R: decompressedR,
                                sigma_i: contract_signers_completed_offline_stage.sigma_i.scalar,
                                pubkey: decompressedPubKey,
                                user_entry_code_hash: USER_ENTRY_CODE_HASH,
                            }
                        }],
                        user_entry_address: USER_ENTRY_ADDRESS,
                    }
                }
                console.log(JSON.stringify(setShareMsg))
                assert(secretjs instanceof SecretNetworkClient);
                let tx = await secretjs.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: secretjs.address, contract_address: contract, msg: setShareMsg,
                }, { gasLimit: 1_000_000 });

                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);

                console.log("Set key gas:", tx.gasUsed);

                // compute the Ethereum UserOp (ERC4337) hash
                let hash = createUserOperationHash(exampleEthUserOp, '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789', '5')
                //console.log(hash);
                expect(hash).to.equal(TEST_USER_OP_HASH);
                const hashByteArray = new Uint8Array(Buffer.from(hash.slice(2), 'hex'));
                // sign the eth signature with all but one of the signers locally
                var partialSignatures: any[] = [];
                for (let signersIndex = 1; signersIndex < signers.length; signersIndex++) {
                    partialSignatures.push(signers[signersIndex].partial(hashByteArray).scalar);
                    //console.log(JSON.stringify(partialSignatures[partialSignatures.length - 1]))
                }
                let execute_sign_msg = {
                    sign: {
                        participants: participatingPartyIds,
                        user_entry_address: USER_ENTRY_ADDRESS,
                        user_entry_code_hash: USER_ENTRY_CODE_HASH,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: partialSignatures,
                    }
                }
                console.log("execute_sign_msg:\n" + JSON.stringify(execute_sign_msg))

                // By executing the contract with the sign message have the contract complete the signature with the last
                // remaining partial signature and return the full signature
                assert(secretjs instanceof SecretNetworkClient);
                tx = await secretjs.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: secretjs.address, contract_address: contract, msg: execute_sign_msg
                }, {
                    gasLimit: 2_000_000
                });

                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);
                console.log("Sign gas:", tx.gasUsed);
                let sig: SignatureRecid = utf8ArrayToObject(MsgExecuteContractResponse.decode(tx.data[0]).data)
                console.log("Final signature from execute contract:\n" + JSON.stringify(sig))
                const publicKey = secp256k1.keyFromPublic(compressedPubKey, 'hex');

                const y = publicKey.getPublic();
                expect(verify(secp256k1, sig, y, new BN(hashByteArray, 16))).to.equal(true);

                // Now by querying the contract with the sign message have the contract complete the signature with the last
                // remaining partial signature and return the full signature
                let query_sign_msg = {
                    sign_userop: {
                        participants: participatingPartyIds,
                        user_entry_address: USER_ENTRY_ADDRESS,
                        user_entry_code_hash: USER_ENTRY_CODE_HASH,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: partialSignatures,
                        userop_signed_by_signers: [userop_signature_by_pubkey_hex],
                    }
                }
                console.log("query_sign_msg:\n" + JSON.stringify(query_sign_msg))
                const querySig: SignatureRecid = await secretjs.query.compute.queryContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    contract_address: contract, query: query_sign_msg
                });
                console.log("Final signature from query contract:\n" + JSON.stringify(querySig))
                expect(querySig).to.deep.equal(sig);
            });

            it('cannot query sign as invalid party', async () => {
                const partiesKeyShares = keygen(keygenParams);
                const participatingPartyIds = [1, 2];
                const signers = signing_offline_stage(partiesKeyShares, participatingPartyIds, keygenAndSignMode);
                // const signers = create_signers_and_pre_sign(partiesKeyShares.map((keyshare) => keyshare.localKey), participatingPartyIds);

                const contract_signer = signers[0];
                const contract_signers_completed_offline_stage = contract_signer.completedOfflineStage();

                const compressedR = contract_signers_completed_offline_stage.R.point;
                const decompressedR = decompressPoint(compressedR);
                const compressedPubKey = contract_signers_completed_offline_stage.local_key.y_sum_s.point;
                const decompressedPubKey = decompressPoint(compressedPubKey);

                // store key share in contract
                let setShareMsg = {
                    set_shares: {
                        participants_to_completed_offline_stages: [{
                            participants: participatingPartyIds, completed_offline_stage: {
                                k_i: contract_signers_completed_offline_stage.sign_keys.k_i.scalar,
                                R: decompressedR,
                                sigma_i: contract_signers_completed_offline_stage.sigma_i.scalar,
                                pubkey: decompressedPubKey,
                                user_entry_code_hash: USER_ENTRY_CODE_HASH,
                            }
                        }],
                        user_entry_address: USER_ENTRY_ADDRESS,
                    }
                }
                assert(accounts[FIRST_OWNER].client instanceof SecretNetworkClient);
                let tx = await accounts[FIRST_OWNER].client.tx.compute.executeContract({
                    code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                    sender: accounts[FIRST_OWNER].client.address, contract_address: contract, msg: setShareMsg,
                }, { gasLimit: 1_000_000 });
                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);

                console.log("Set key gas:", tx.gasUsed);

                // compute the Ethereum UserOp (ERC4337) hash
                let hash = createUserOperationHash(exampleEthUserOp, '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789', '5')
                //console.log(hash);
                expect(hash).to.equal(TEST_USER_OP_HASH);
                const hashByteArray = new Uint8Array(Buffer.from(hash.slice(2), 'hex'));
                // sign the eth signature with all but one of the signers locally
                var partialSignatures: any[] = [];
                for (let signersIndex = 1; signersIndex < signers.length; signersIndex++) {
                    partialSignatures.push(signers[signersIndex].partial(hashByteArray).scalar);
                    //console.log(JSON.stringify(partialSignatures[partialSignatures.length - 1]))
                }

                // Query sign with the permit from the wrong account it should fail
                try {
                    assert(accounts[2].wallet instanceof Wallet);
                    let querySignWithDifAccountResult = await accounts[FIRST_OWNER].client.query.compute.queryContract({
                        code_hash: contractDict.getFlattenedAttributes("secret_share_signer")!.codeHash!,
                        contract_address: contract, query: {
                            sign_userop: {
                                participants: participatingPartyIds,
                                user_entry_address: contractDict.getFlattenedAttributes("user_entry", CHAINID)!.contractAddress!,
                                user_entry_code_hash: contractDict.getFlattenedAttributes("user_entry")!.codeHash!,
                                entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                                chain_id: "5",
                                user_operation: transformEthUserOp(exampleEthUserOp),
                                other_partial_sigs: partialSignatures,
                                userop_signed_by_signers: [bad_userop_signature_by_pubkey_hex],
                            }
                        }
                    });
                    assert(false, "Query sign with the permit from the wrong account it should fail");
                } catch (e) {
                    console.log("Query failed (as it should) with error " + JSON.stringify(e));
                }
            });
            it('cannot execute sign as invalid party', async () => {
                const partiesKeyShares = keygen(keygenParams);
                const participatingPartyIds = [1, 2];
                const signers = signing_offline_stage(partiesKeyShares, participatingPartyIds, keygenAndSignMode);
                // const signers = create_signers_and_pre_sign(partiesKeyShares.map((keyshare) => keyshare.localKey), participatingPartyIds);

                const contract_signer = signers[0];
                const contract_signers_completed_offline_stage = contract_signer.completedOfflineStage();
                //console.log(JSON.stringify(contract_signers_completed_offline_stage));

                const compressedR = contract_signers_completed_offline_stage.R.point;
                const decompressedR = decompressPoint(compressedR);
                const compressedPubKey = contract_signers_completed_offline_stage.local_key.y_sum_s.point;
                const decompressedPubKey = decompressPoint(compressedPubKey);

                // store key share in contract
                const { client: secretjs } = accounts[FIRST_OWNER];
                let setShareMsg = {
                    set_shares: {
                        participants_to_completed_offline_stages: [{
                            participants: participatingPartyIds, completed_offline_stage: {
                                k_i: contract_signers_completed_offline_stage.sign_keys.k_i.scalar,
                                R: decompressedR,
                                sigma_i: contract_signers_completed_offline_stage.sigma_i.scalar,
                                pubkey: decompressedPubKey,
                                user_entry_code_hash: USER_ENTRY_CODE_HASH,
                            }
                        }],
                        user_entry_address: USER_ENTRY_ADDRESS,
                    }
                }
                console.log(JSON.stringify(setShareMsg))
                assert(secretjs instanceof SecretNetworkClient);
                let tx = await secretjs.tx.compute.executeContract({
                    sender: secretjs.address, contract_address: contract, msg: setShareMsg,
                }, { gasLimit: 1_000_000 });

                if (tx.code !== 0) {
                    console.log(tx.rawLog);
                }
                expect(tx.code).to.equal(0);

                console.log("Set key gas:", tx.gasUsed);

                // compute the Ethereum UserOp (ERC4337) hash
                let hash = createUserOperationHash(exampleEthUserOp, '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789', '5')
                //console.log(hash);
                expect(hash).to.equal(TEST_USER_OP_HASH);
                const hashByteArray = new Uint8Array(Buffer.from(hash.slice(2), 'hex'));
                // sign the eth signature with all but one of the signers locally
                var partialSignatures: any[] = [];
                for (let signersIndex = 1; signersIndex < signers.length; signersIndex++) {
                    partialSignatures.push(signers[signersIndex].partial(hashByteArray).scalar);
                    //console.log(JSON.stringify(partialSignatures[partialSignatures.length - 1]))
                }
                let execute_sign_msg = {
                    sign: {
                        participants: participatingPartyIds,
                        user_entry_address: USER_ENTRY_ADDRESS,
                        user_entry_code_hash: USER_ENTRY_CODE_HASH,
                        entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                        chain_id: "5",
                        user_operation: transformEthUserOp(exampleEthUserOp),
                        other_partial_sigs: partialSignatures,
                    }
                }
                console.log("execute_sign_msg:\n" + JSON.stringify(execute_sign_msg))

                // By executing the contract with the sign message have the contract complete the signature with the last
                // remaining partial signature and return the full signature
                try {
                    assert(accounts[0].client instanceof SecretNetworkClient);
                    tx = await accounts[0].client.tx.compute.executeContract({
                        sender: accounts[0].address, contract_address: contract, msg: execute_sign_msg
                    }, {
                        gasLimit: 2_000_000
                    });

                    if (tx.code === 0) {
                        console.log(tx.rawLog);
                    }
                    expect(tx.code).to.not.equal(0);
                } catch (e) {
                    //success
                    console.log("Execute failed as expected. Error: " + e);
                }
            });
        }
    });
});