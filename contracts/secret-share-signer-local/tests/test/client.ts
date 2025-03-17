import { Wallet, SecretNetworkClient, MsgInstantiateContractParams } from "secretjs";
import * as fs from "fs";
import axios from "axios";
import { ContractDict, ContractDeployment } from "./contract_dict";
import { loadEnvVars } from "./helpers";
import * as crypto from "crypto";
import assert from "assert";

// Use the NODE_ENV environment variable, or default to 'development'
const env = process.env.NODE_ENV || "development";
loadEnvVars(env);

export const makeFlexAccount = async (endpoint: string, chainId: string) => {
  return initializeClient(endpoint, chainId, true);
};

// Returns a client with which we can interact with secret network
export const initializeClient = async (
  endpoint: string,
  chainId: string,
  random: boolean,
  mnemonic?: string
) => {
  const wallet = random
    ? new Wallet()
    : new Wallet(mnemonic ? mnemonic : process.env.MNEMONIC);
  // Use default constructor of wallet to generate random mnemonic.
  const accAddress = wallet.address;
  const client = new SecretNetworkClient({
    // Create a client to interact with the network
    url: endpoint,
    chainId: chainId,
    wallet: wallet,
    walletAddress: accAddress,
  });

  const accountTypeString = random ? `flex account ` : `owner `;

  console.log(
    `Initialized ` +
      accountTypeString +
      `client with wallet address: ${accAddress}`
  );
  return client;
};

export const initializeContract = async (
  client: SecretNetworkClient,
  contractPath: string
) => {
  const wasmCode = fs.readFileSync(contractPath);
  console.log("Uploading contract at: ", contractPath, "...");

  const uploadReceipt = await client.tx.compute.storeCode(
    {
      wasm_byte_code: wasmCode,
      sender: client.address,
      source: "",
      builder: "",
    },
    {
      gasLimit: 3510000,
    }
  );

  if (uploadReceipt.code !== 0) {
    console.log(
      `Failed to get code id: ${JSON.stringify(uploadReceipt.rawLog)}`
    );
    throw new Error(`Failed to upload contract`);
  }

  const codeIdKv = uploadReceipt.jsonLog![0].events[0].attributes.find(
    (a: any) => {
      return a.key === "code_id";
    }
  );

  const codeId = Number(codeIdKv!.value);
  // console.log("Contract codeId: ", codeId);

  const contractCodeHash = (
    await client.query.compute.codeHashByCodeId({ code_id: String(codeId) })
  ).code_hash;

  if (contractCodeHash === undefined) {
    throw new Error(`Failed to get code hash`);
  }

  // console.log(`Contract hash: ${contractCodeHash}`);

  let codeIdNum: number = codeId.valueOf();
  return { codeId: codeIdNum, codeHash: contractCodeHash };
};

export const getFromFaucet = async (address: string) => {
  await axios.get(`${process.env.FAUCET}?address=${address}`);
};

export async function getScrtBalance(
  userCli: SecretNetworkClient
): Promise<string> {
  let balanceResponse = await userCli.query.bank.balance({
    address: userCli.address,
    denom: "uscrt",
  });

  if (balanceResponse.balance?.amount === undefined) {
    throw new Error(`Failed to get balance for address: ${userCli.address}`);
  }

  return balanceResponse.balance.amount;
}

export async function fillUpFromFaucet(
  client: SecretNetworkClient,
  targetBalance: Number
) {
  let balance = await getScrtBalance(client);
  while (Number(balance) < Number(targetBalance)) {
    try {
      await getFromFaucet(client.address);
    } catch (e) {
      console.error(`failed to get tokens from faucet: ${e}`);
    }
    balance = await getScrtBalance(client);
  }
  console.log(`got tokens from faucet: ${balance}`);
}

export async function initializeClientWithEnv(mnemonic?: string) {
  let endpoint = process.env.ENDPOINT;
  let chainId = process.env.CHAINID;
  let client: SecretNetworkClient;
  if (endpoint && chainId) {
    client = await initializeClient(endpoint, chainId, false, mnemonic);
  } else {
    throw new Error(`Missing environment variables`);
  }
  return client;
}

// Initialization procedure
export async function initializeAndUploadContracts(
  noInit: boolean,
  mnemonic: string,
  codeDetailsPath: string
) {
  try {
    let chainId = process.env.CHAINID!;
    let client = await initializeClientWithEnv(mnemonic);

    // We can fill up ahead of time; don't need it here
    // await fillUpFromFaucet(client, 100_000_000);

    // Import JSON file
    let rawContractDict = fs.readFileSync(codeDetailsPath, "utf-8");
    let contractDict = new ContractDict(JSON.parse(rawContractDict));

    if (!noInit) {
      for (const contract of contractDict.contracts) {
        // find the process.env.CHAINID deployment
        // create it if it doesn't exist
        let skip = false;
        if (contract.deployments === undefined) {
          contract.deployments = [];
        }
        let deployment = contract.deployments.find(
          (d) => d.chainId === chainId
        );
        if (deployment === undefined) {
          console.log(
            "No deployment found for chainId: " +
              chainId +
              ", creating new deployment"
          );
          deployment = new ContractDeployment(chainId);
        } else if (deployment.codeId !== undefined) {
          // if deployment exists, only upload if code has changed
          // check sha256 hash of the local wasm file
          const wasmCode = `../../../optimized-wasm/${contract.name}.wasm.gz`
          // calculate the sha256 hash
          const localCodeHash = crypto
            .createHash("sha256")
            .update(wasmCode)
            .digest("hex");
          if (localCodeHash === contract.codeHash) {
            // if testnet or localnet has reset, we still need to reupload even if no changes
            try {
              // wait 5 seconds
              await new Promise((r) => setTimeout(r, 5000));
              const code = await client.query.compute.code({
                code_id: String(deployment.codeId),
              });
              console.log(
                "Code for contract " +
                  contract.name +
                  " has not changed, skipping upload"
              );
              // wait one second
              await new Promise((r) => setTimeout(r, 1000));
              skip = true;
            } catch {
              skip = false;
            }
          }
        }
        if (!skip) {
          // clear contractAddress since we're updating code
          deployment.contractAddress = undefined;
          const contractPath = 
            `../../../optimized-wasm/${contract.name}.wasm.gz`;
          const { codeId, codeHash } = await initializeContract(
            client,
            contractPath
          );
          contract.add_code(codeId, codeHash, chainId);
          // Update JSON file
          fs.writeFile(
            codeDetailsPath,
            JSON.stringify(contractDict.contracts, null, 2),
            "utf-8",
            (error) => {
              if (error) {
                console.error("Error writing file", error);
              } else {
                console.log("Code id & hash saved to " + codeDetailsPath);
              }
            }
          );
        }
      }
    }

    var clientInfo: [SecretNetworkClient, ContractDict] = [
      client,
      contractDict,
    ];
    return clientInfo;
  } catch (error) {
    throw new Error(`Error initializing client: ${JSON.stringify(error)}`);
  }
}

export async function instantiateContract(
  client: SecretNetworkClient,
  contractDict: ContractDict,
  contractName: string,
  initMsg: object,
  codeDetailsPath: string,
  admin?: string
) {
  const chainId = process.env.CHAINID;
  assert(chainId !== undefined, "CHAINID environment variable not set");

  const thisContract = contractDict.contracts.find(
    (contract) => contract.name === contractName
  );
  if (thisContract === undefined) {
    throw new Error(`Contract ${contractName} not found in contractDict`);
  } else {
    // find the deployment matching process.env.CHAINID
    const deployment = thisContract.deployments?.find(
      (deployment) => deployment.chainId === chainId
    );
    const codeId = deployment?.codeId;
    const codeHash = thisContract.codeHash;
    assert(codeId !== undefined, `Code ID for ${contractName} not found`);
    const initArgs: MsgInstantiateContractParams = {
      sender: client.address,
      code_id: codeId,
      init_msg: initMsg,
      code_hash: codeHash,
      label: contractName + Math.ceil(Math.random() * 10000),
      // admin: admin
    };
    console.log(
      `Instantiating contract ${contractName} with init args: \x1b[37m${JSON.stringify(
        initArgs
      )}\x1b[0m`
    );
    const contract = await client.tx.compute.instantiateContract(initArgs, {
      gasLimit: 1000000,
    });

    if (contract.code !== 0) {
      throw new Error(
        `Failed to instantiate the contract with the following error ${contract.rawLog}`
      );
    }

    const contractAddress = contract.arrayLog!.find(
      (log) => log.type === "message" && log.key === "contract_address"
    )!.value;
    contractDict.contracts
      .find((contract) => contract.name === contractName)!
      .add_instance(contractAddress, chainId);
    // write the updated file
    fs.writeFile(
      codeDetailsPath,
      JSON.stringify(contractDict.contracts, null, 2),
      "utf-8",
      (error) => {
        if (error) {
          console.error("Error writing file", error);
        } else {
          // console.log("Address saved to " + codeDetailsPath);
        }
      }
    );
    return contractAddress;
  }
}
