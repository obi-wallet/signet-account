import {Signer} from "@mpc-sdk/mpc-bindings";
import { LocalKey } from "./mpc-ecdsa-wasm-types";

export function create_signers_and_pre_sign(local_keys: LocalKey[], selectedPartyIds: number[]): Signer[] {
  let signers: Signer[] = [];
  for (let selectedPartyIdx = 0; selectedPartyIdx < selectedPartyIds.length; selectedPartyIdx++) {
    let signer = new Signer(
      selectedPartyIdx + 1,
      selectedPartyIds,
      local_keys[selectedPartyIds[selectedPartyIdx] - 1]
    );
    console.log(`signer ${selectedPartyIdx + 1} created with localkey: ${local_keys[selectedPartyIds[selectedPartyIdx] - 1]}`);
    signers.push(signer);
  }
  let partyToOutgoingRoundMsgs: { [key: number]: any[] } = {};

  function handleRound() {
    if (Object.keys(partyToOutgoingRoundMsgs).length !== 0) {
      for (let selectedPartyId of selectedPartyIds) {
        for (let party_round_msg_idx = 0; party_round_msg_idx < partyToOutgoingRoundMsgs[selectedPartyId].length; party_round_msg_idx++) {
          if (partyToOutgoingRoundMsgs[selectedPartyId][party_round_msg_idx].receiver != null) {
            let outgoingRoundMsgWithRecipient = partyToOutgoingRoundMsgs[selectedPartyId][party_round_msg_idx] as any;
            let keygenIndex = Number(outgoingRoundMsgWithRecipient.receiver) - 1;
            // console.log(`presign ${keygenIndex} handleIncoming round:${outgoingRoundMsgWithRecipient.round} msg sender:${outgoingRoundMsgWithRecipient.sender} receiver:${outgoingRoundMsgWithRecipient.receiver})`);
            signers[keygenIndex].handleIncoming(outgoingRoundMsgWithRecipient);
          } else {
            let roundMsgWithoutRecipient = partyToOutgoingRoundMsgs[selectedPartyId][0] as any;
            for (let receiving_party_keygen_idx = 0; receiving_party_keygen_idx < selectedPartyIds.length; receiving_party_keygen_idx++) {
              if (receiving_party_keygen_idx + 1 !== roundMsgWithoutRecipient.sender) {
                // console.log(`presign ${receiving_party_keygen_idx + 1} handleIncoming round:${roundMsgWithoutRecipient.round} msg sender:${roundMsgWithoutRecipient.sender} receiver:${roundMsgWithoutRecipient.receiver})`);
                signers[receiving_party_keygen_idx].handleIncoming(roundMsgWithoutRecipient);
              }
            }
          }
        }
      }
    }

    for (let selectedPartyIdx = 0; selectedPartyIdx < selectedPartyIds.length; selectedPartyIdx++) {
      let signer = signers[selectedPartyIdx];
      let result = signer.proceed();
      // index 1 of result is an array of outgoing round messages from the party that should be sent to other parties
      let roundOutgoingMessages = result[1];
      // console.log(`proceed result${result} signerIdx${selectedPartyIdx} round ${result[0]} outgoing messages ${roundOutgoingMessages.length}`)
      if (selectedPartyIdx === 0 && roundOutgoingMessages.length > 0) {
        // index 0 contains the round number
        console.log(`start round ${result[0]}`);
      }
      partyToOutgoingRoundMsgs[selectedPartyIds[selectedPartyIdx]] = result[1];
    }

    // Check if result[1] is an empty array
    // If so, then we are done. Do not proceed to next round.
    if (!Object.values(partyToOutgoingRoundMsgs).every(msgs => msgs.length === 0)) {
      handleRound();
    }
  }

  handleRound();
  console.log("completed presign");
  return signers;
}
