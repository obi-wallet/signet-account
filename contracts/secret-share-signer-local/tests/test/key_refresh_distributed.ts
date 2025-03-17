import {Parameters} from "./mpc-ecdsa-wasm-types.js";
import {KeyRefresh} from "@mpc-sdk/mpc-bindings";

export type ExistingKeyRefreshItem = {
    Existing: {
        key: any;
        updated_party_index?: number;
    };
}

export type NewKeyRefreshItem = {
    New: {
        party_index: number;
    };
}

export type KeyRefreshItem = ExistingKeyRefreshItem | NewKeyRefreshItem;

export type Pair = [number, number];


export function keyRefreshDistributed(params: Parameters, keyRefreshItems: KeyRefreshItem[]): any[] {
    let keyRefreshes: KeyRefresh[] = [];
    let old_to_new: number[][] = [];
    let old_t = 0;
    for (let item of keyRefreshItems) {
        if ('Existing' in item) {
            // ExistingKeyRefreshItem
            let existing = (item as ExistingKeyRefreshItem).Existing;
            let new_party_index;
            if (existing.updated_party_index != undefined) {
                new_party_index = existing.updated_party_index;
            } else {
                new_party_index = existing.key.i;
            }
            old_to_new.push([existing.key.i, new_party_index!]);
            old_t = existing.key.t;
        }
    }
    for (let item of keyRefreshItems) {
        let keyRefresh: KeyRefresh;
        if ('Existing' in item) {
            let existing = (item as ExistingKeyRefreshItem).Existing;
            keyRefresh = new KeyRefresh(
                params,
                existing.key,
                undefined,
                old_to_new,
                undefined
            );
        } else {
            let newItem = (item as NewKeyRefreshItem).New;
            keyRefresh = new KeyRefresh(
                params,
                undefined,
                newItem.party_index,
                old_to_new,
                old_t
            );
        }
        keyRefreshes.push(keyRefresh);
    }

    let partyToOutgoingRoundMsgs: { [key: number]: any[] } = {};
    console.log(`keyRefreshDistributed: KeyRefresh creation for all done`)

    function handleRound() {
        if (Object.keys(partyToOutgoingRoundMsgs).length !== 0) {
            for (let party_signer_idx = 0; party_signer_idx < params.parties; party_signer_idx++) {
                for (let party_round_msg_idx = 0; party_round_msg_idx < partyToOutgoingRoundMsgs[party_signer_idx].length; party_round_msg_idx++) {
                    console.log(`handleRound: party_signer_idx ${party_signer_idx} party_round_msg_idx ${party_round_msg_idx}`)
                    if (partyToOutgoingRoundMsgs[party_signer_idx][party_round_msg_idx].receiver != null) {
                        let outgoingRoundMsgWithRecipient = partyToOutgoingRoundMsgs[party_signer_idx][party_round_msg_idx] as any;
                        let keyRefreshIndex = Number(outgoingRoundMsgWithRecipient.receiver) - 1;
                        keyRefreshes[keyRefreshIndex].handleIncoming(outgoingRoundMsgWithRecipient);
                    } else {
                        let roundMsgWithoutRecipient = partyToOutgoingRoundMsgs[party_signer_idx][0] as any;
                        for (let receiving_party_key_refresh_idx = 0; receiving_party_key_refresh_idx < params.parties; receiving_party_key_refresh_idx++) {
                            if (receiving_party_key_refresh_idx + 1 !== roundMsgWithoutRecipient.sender) {
                                console.log(`keygen ${receiving_party_key_refresh_idx + 1} handleIncoming round:${roundMsgWithoutRecipient.round} msg sender:${roundMsgWithoutRecipient.sender} receiver:${roundMsgWithoutRecipient.receiver})`);
                                keyRefreshes[receiving_party_key_refresh_idx].handleIncoming(roundMsgWithoutRecipient);
                            }
                        }
                    }
                }
            }
        }

        for (let p = 0; p < params.parties; p++) {
            let result = keyRefreshes[p].proceed();
            // index 1 of result is an array of outgoing round messages from the party that should be sent to other parties
            let roundOutgoingMessages = result[1];
            if (p === 0 && roundOutgoingMessages.length > 0) {
                // index 0 contains the round number
                console.log(`start round ${result[0]}`);
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
        keys.push(keyRefreshes[p].create());
    }
    return keys;
}