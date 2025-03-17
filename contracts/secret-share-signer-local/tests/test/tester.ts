import { SecretNetworkClient } from "secretjs";
import { ContractDict } from "./contract_dict";
import { Payload } from "./integration_setup";

export async function runTestFunction(
  tester: (
    client: SecretNetworkClient,
    contractDict: ContractDict,
    payload: Payload,
    test_log: string
  ) => Promise<string>,
  client: SecretNetworkClient,
  contractDict: ContractDict,
  payload: Payload,
  testLog: string
) {
  let finalTestLog = await tester(client, contractDict, payload, testLog);
  console.log(finalTestLog);
  console.log("[SUCCESS]");
}
