import {SecretNetworkClient, Wallet} from "secretjs";
import {toBase64} from "@cosmjs/encoding";
import { LCDClient, Wallet as LegacyWallet } from "@terra-money/terra.js";

export type AminoPubkey = {
  readonly type: string;
  readonly value: any;
};

export function encodeSecp256k1Pubkey(pubkey: Uint8Array): AminoPubkey {
  if (pubkey.length !== 33 || (pubkey[0] !== 0x02 && pubkey[0] !== 0x03)) {
    throw new Error(
        "Public key must be compressed secp256k1, i.e. 33 bytes starting with 0x02 or 0x03",
    );
  }
  return {
    type: "tendermint/PubKeySecp256k1",
    value: toBase64(pubkey),
  };
}


export async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function waitForBlocks(chainId: string, url: string) {
  const secretjs = new SecretNetworkClient({
    url,
    chainId,
  });

  console.log(`Waiting for blocks on ${chainId}...`);
  while (true) {
    try {
      const {block} = await secretjs.query.tendermint.getLatestBlock({});

      if (Number(block?.header?.height) >= 1) {
        console.log(`Current block on ${chainId}: ${block!.header!.height}`);
        break;
      }
    } catch (e) {
    }
    await sleep(100);
  }
}

export type Account = {
  address: string;
  mnemonic: string;
  wallet: Wallet | LegacyWallet;
  client: SecretNetworkClient | LCDClient;
};
