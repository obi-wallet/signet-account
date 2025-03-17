import fs from "fs";
import dotenv from "dotenv";

export function loadEnvVars(environment: string) {
  const envPath = `.env.${environment}`;
  if (fs.existsSync(envPath)) {
    console.log(`Using .env file: ${envPath}`);
    dotenv.config({ path: envPath });
  } else {
    console.log(`.env file not found: ${envPath}`);
  }
}

export function findContractAddress(
  logs: any[],
  codeId: string
): string | undefined {
  for (let i = 0; i < logs.length; i++) {
    if (logs[i].key === "code_id" && logs[i].value === codeId) {
      if (i + 1 < logs.length && logs[i + 1].key === "contract_address") {
        return logs[i + 1].value;
      }
    }
  }
  return undefined; // return undefined if no matching contract_address found
}

export const jsonToBase64 = async (json: object): Promise<string> => {
  const jsonString = JSON.stringify(json);
  return Buffer.from(jsonString).toString("base64");
};

export const base64ToJson = async (base64: string): Promise<object> => {
  const jsonString = Buffer.from(base64, "base64").toString();
  return JSON.parse(jsonString);
};

export function hexToByteArrayJson(hexString: string) {
  let byteArray: number[] = [];
  for (let i = 0; i < hexString.length; i += 2) {
    const hexByte = hexString.slice(i, i + 2);
    const byte = parseInt(hexByte, 16);
    if (isNaN(byte)) {
      throw new Error("Invalid hex byte: " + hexByte);
    } else {
      byteArray.push(byte);
    }
  }
  return byteArray;
}
export function assembleEthCallData(
  recipient: string,
  amount: string,
  denom: string,
  multicall: boolean,
  feeOverride?: number,
  feePayAddressOveride?: string,
  tokenOverride?: string
): string {
  console.log("assembling user op with payment amount: " + amount);
  if (multicall) {
    let callData = "0x18dfb3c7"
      + "0000000000000000000000000000000000000000000000000000000000000040"
      + "00000000000000000000000000000000000000000000000000000000000000a0"
      + "0000000000000000000000000000000000000000000000000000000000000002"
      + "000000000000000000000000"
      + (tokenOverride ? tokenOverride : "5cf29823ccfc73008fa53630d54a424ab82de6f2")
      + "000000000000000000000000"
      + (tokenOverride ? tokenOverride : "5cf29823ccfc73008fa53630d54a424ab82de6f2")
      + "0000000000000000000000000000000000000000000000000000000000000002"
      + "0000000000000000000000000000000000000000000000000000000000000040"
      + "00000000000000000000000000000000000000000000000000000000000000c0"
      + "0000000000000000000000000000000000000000000000000000000000000044"
      + "a9059cbb000000000000000000000000"
      + (recipient.substring(2)) //652
      + (parseInt(amount) as any).toString(16).padStart(64, '0')
      + "00000000000000000000000000000000000000000000000000000000"
      + "0000000000000000000000000000000000000000000000000000000000000044"
      + "a9059cbb000000000000000000000000"
      + (feePayAddressOveride ? feePayAddressOveride.substring(2) : "c1d4f3dcc31d86a66de90c3c5986f0666cc52ce4")
      + (feeOverride ? (feeOverride as any).toString(16).padStart(64, '0') : ((Math.floor(parseInt(amount) / 1000)) as any).toString(16).padStart(64, '0'))
      + "00000000000000000000000000000000000000000000000000000000";
    return callData;
  } else {
    //console.log("assembling eth call data for recipient: " + recipient + ", amount: " + amount + ", denom: " + denom);
    const transfer: string = "b61d27f6000000000000000000000000";
    const token_contract: string =
      denom.substring(0, 2) === "0x" ? denom.substring(2) : denom;

    const pad_102: string =
      "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    const unknown: string =
      "600000000000000000000000000000000000000000000000000000000000000044a9059cbb000000000000000000000000";
    const recipient_address: string =
      recipient.substring(0, 2) === "0x" ? recipient.substring(2) : recipient;
    const hex_amount: string = parseInt(amount).toString(16);
    let padded_amount: string = hex_amount;
    for (let i = 0; i < 64 - hex_amount.length; i++) {
      padded_amount = "0" + padded_amount;
    }
    const end_pad: string =
      "00000000000000000000000000000000000000000000000000000000";

    const combined: string =
      transfer +
      token_contract +
      pad_102 +
      unknown +
      recipient_address +
      padded_amount +
      end_pad;
    //console.log("assembled call data: " + combined);
    return combined;
  }
}
