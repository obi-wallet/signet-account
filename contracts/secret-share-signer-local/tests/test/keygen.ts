import {LocalKey, Parameters, PartySignup} from "./mpc-ecdsa-wasm-types.js";
import {KeyGenerator} from "@mpc-sdk/mpc-bindings";

export function keygen(params: Parameters): any[] {
  let keygens: KeyGenerator[] = [];
  for (let p = 1; p <= params.parties; p++) {
    const party: PartySignup = {
      number: p,
      uuid: "some-uuid"
    };
    let keygen = new KeyGenerator(
      params,
      party
    );
    keygens.push(keygen);
  }
  let partyToOutgoingRoundMsgs: { [key: number]: any[] } = {};

  function handleRound() {
    if (Object.keys(partyToOutgoingRoundMsgs).length !== 0) {
      for (let party_signer_idx = 0; party_signer_idx < params.parties; party_signer_idx++) {
        for (let party_round_msg_idx = 0; party_round_msg_idx < partyToOutgoingRoundMsgs[party_signer_idx].length; party_round_msg_idx++) {
          if (partyToOutgoingRoundMsgs[party_signer_idx][party_round_msg_idx].receiver != null) {
            let outgoingRoundMsgWithRecipient = partyToOutgoingRoundMsgs[party_signer_idx][party_round_msg_idx] as any;
            let keygenIndex = Number(outgoingRoundMsgWithRecipient.receiver) - 1;
            // console.log(`keygen ${keygenIndex} handleIncoming round:${outgoingRoundMsgWithRecipient.round} msg sender:${outgoingRoundMsgWithRecipient.sender} receiver:${outgoingRoundMsgWithRecipient.receiver})`);
            keygens[keygenIndex].handleIncoming(outgoingRoundMsgWithRecipient);
          } else {
            let roundMsgWithoutRecipient = partyToOutgoingRoundMsgs[party_signer_idx][0] as any;
            for (let receiving_party_keygen_idx = 0; receiving_party_keygen_idx < params.parties; receiving_party_keygen_idx++) {
              if (receiving_party_keygen_idx + 1 !== roundMsgWithoutRecipient.sender) {
                // console.log(`keygen ${receiving_party_keygen_idx + 1} handleIncoming round:${roundMsgWithoutRecipient.round} msg sender:${roundMsgWithoutRecipient.sender} receiver:${roundMsgWithoutRecipient.receiver})`);
                keygens[receiving_party_keygen_idx].handleIncoming(roundMsgWithoutRecipient);
              }
            }
          }
        }
      }
    }

    for (let p = 0; p < params.parties; p++) {
      let result = keygens[p].proceed();
      // index 1 of result is an array of outgoing round messages from the party that should be sent to other parties
      let roundOutgoingMessages = result[1];
      if (p === 0 && roundOutgoingMessages.length > 0) {
        // index 0 contains the round number
        // console.log(`start round ${result[0]}`);
      }
      partyToOutgoingRoundMsgs[p] = result[1];
    }

    // Check if result[1] is an empty array
    // If so, then we are done. Do not proceed to next round.
    if (!Object.values(partyToOutgoingRoundMsgs).every(msgs => msgs.length === 0)) {
      handleRound();
    }
  }

  handleRound();

  let keys: any[] = [];
  for (let p = 0; p < params.parties; p++) {
    keys.push(keygens[p].create());
  }
  return keys;
}
