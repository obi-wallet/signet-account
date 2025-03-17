import React, { useEffect } from "react";
import { useSelector, useDispatch } from "react-redux";
import {
  ButtonGroup,
  Button,
  List,
  ListSubheader,
  ListItem,
  ListItemText,
} from "@mui/material";
import { formatDistanceToNow } from "date-fns";

import { SignProof, SignMessage } from "../types";
import { encode, download, toHexString, sortTimestamp } from "../utils";
import { AppDispatch } from "../store";
import { loadMessageProofs, proofsSelector } from "../store/proofs";
import {
  setDialogVisible,
  CONFIRM_DELETE_MESSAGE_PROOF,
} from "../store/dialogs";

type MessageProofProps = {
  address: string;
};

export default function MessageProofs(props: MessageProofProps) {
  const dispatch: AppDispatch = useDispatch();
  const { address } = props;
  const { messages } = useSelector(proofsSelector);
  // NOTE: need a copy of the array as we need to sort in place
  const items: SignProof[] = (messages[address] || []).slice(0);

  useEffect(() => {
    const getMessageProofs = async () => {
      await dispatch(loadMessageProofs());
    };
    getMessageProofs();
  }, []);

  const onExportMessageProof = (proof: SignProof) => {
    const dt = new Date();
    dt.setTime(proof.timestamp);
    const fileName = `${address}-${toHexString(
      proof.value.digest
    )}-${dt.toISOString()}.json`;
    const buffer = encode(JSON.stringify(proof, undefined, 2));
    download(fileName, buffer);
  };

  const onDeleteMessageProof = (proof: SignProof) => {
    dispatch(
      setDialogVisible([CONFIRM_DELETE_MESSAGE_PROOF, true, [address, proof]])
    );
  };

  if (items.length === 0) {
    return null;
  }

  return (
    <List
      component="div"
      subheader={<ListSubheader component="div">Proofs</ListSubheader>}
    >
      {items.sort(sortTimestamp).reverse().map((proof: SignProof, index: number) => {
        const formatDateDistance = () => {
          const dt = new Date();
          dt.setTime(proof.timestamp);
          return formatDistanceToNow(dt);
        };

        return (
          <ListItem key={index}>
            <ListItemText secondary={`Approved ${formatDateDistance()} ago`}>
              {(proof.value as SignMessage).message}
            </ListItemText>
            <ButtonGroup
              variant="outlined"
              size="small"
              aria-label="message proof actions"
            >
              <Button onClick={() => onExportMessageProof(proof)}>
                Export
              </Button>
              <Button color="error" onClick={() => onDeleteMessageProof(proof)}>
                Delete
              </Button>
            </ButtonGroup>
          </ListItem>
        );
      })}
    </List>
  );
}
