import React, { useState } from "react";
import { useParams } from "react-router-dom";
import { useSelector } from "react-redux";

import { Button, Stack } from "@mui/material";

import { StepProps } from "./";
import { sessionSelector } from "../../store/session";
import { SigningType, SignMessage, SignTransaction } from "../../types";
import InviteCard, { inviteHref } from "../invite-card";
import SignMessageView from "./message-view";
import SignTransactionView from "./transaction-view";
import Approval from "./approval";

type InvitePeopleProps = {
  kind: SigningType;
} & StepProps;

export default function InvitePeople(props: InvitePeopleProps) {
  const { address } = useParams();
  const [approved, setApproved] = useState(false);
  const { next } = props;
  const { group, session, signCandidate } = useSelector(sessionSelector);

  const kind = props.kind == SigningType.MESSAGE ? "message" : "transaction";

  const totalInvites = group.params.parties - 1;
  const hrefPrefix = `keys/${address}/sign/join/${kind}`;
  const href = inviteHref(hrefPrefix, group.uuid, session.uuid);
  const links = Array(totalInvites)
    .fill("")
    .map(() => href);

  let view;

  if (props.kind == SigningType.MESSAGE) {
    const { message, digest } = signCandidate.value as SignMessage;
    view = <SignMessageView message={message} digest={digest} />;
  } else {
    const { transaction, digest } = signCandidate.value as SignTransaction;
    view = <SignTransactionView transaction={transaction} digest={digest} />;
  }

  const onApprove = () => setApproved(true);

  if (!approved) {
    return (
      <Stack spacing={4}>
        {view}
        <Approval signingType={props.kind} onApprove={onApprove} />
      </Stack>
    );
  } else {
    return (
      <Stack spacing={4}>
        {view}
        <InviteCard links={links} />
        <Button variant="contained" onClick={next}>Next</Button>
      </Stack>
    );
  }

}
