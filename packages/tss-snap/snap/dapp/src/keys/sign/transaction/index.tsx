import React, { useState, useContext, useEffect } from "react";
import { useParams } from "react-router-dom";
import { useSelector, useDispatch } from "react-redux";

import { Box, Chip, Breadcrumbs, Link, Stack, Typography } from "@mui/material";

import { SessionKind } from "@lavamoat/mpc-client";

import { SigningType, SignTransaction } from "../../../types";

import { WebSocketContext, ListenerCleanup } from "../../../websocket-provider";
import { createGroupSession, GroupFormData } from "../../../group-session";

import NotFound from "../../../not-found";
import PublicAddress from "../../../components/public-address";
import { keysSelector, KeyShareGroup } from "../../../store/keys";
import { setSnackbar } from "../../../store/snackbars";
import {
  sessionSelector,
  setSignCandidate,
  clearSign,
} from "../../../store/session";
import SignStepper from "../../../components/stepper";
import KeysLoader from "../../loader";

import { StepProps } from "../";
import CreateTransaction from "./create-transaction";

import InvitePeople from "../invite-people";
import Compute from "../compute";
import SendTransaction from "../send-transaction";

import Signer from "../signer";

import { ChooseKeyShareProps } from "../choose-key-share";

const steps = ["Create transaction", "Invite people", "Compute", "Send Transaction"];

const getStepComponent = (activeStep: number, props: SignTransactionProps) => {
  const stepComponents = [
    <CreateTransaction key={0} {...props} />,
    <InvitePeople key={1} {...props} kind={SigningType.TRANSACTION} />,
    <Compute key={2} {...props} />,
    <SendTransaction key={3} {...props} />,
  ];
  return stepComponents[activeStep];
};

export type SignTransactionProps = {
  onTransaction: (tx: SignTransaction) => void;
} & StepProps &
  ChooseKeyShareProps;

export default function SignMessage() {
  const dispatch = useDispatch();
  const websocket = useContext(WebSocketContext);

  const { address } = useParams();
  const { keyShares, loaded } = useSelector(keysSelector);
  const { signProof } = useSelector(sessionSelector);
  const [activeStep, setActiveStep] = useState(0);
  const [selectedParty, setSelectedParty] = useState(null);

  useEffect(() => {
    // Clear any previous signature data
    dispatch(clearSign());
  }, []);

  if (!loaded) {
    return <KeysLoader />;
  }

  const keyShare = keyShares.find((item) => {
    const [keyAddress] = item;
    return keyAddress === address;
  });

  if (!keyShare) {
    return <NotFound />;
  }

  const handleNext = () => {
    setActiveStep((prevActiveStep) => prevActiveStep + 1);
  };

  const keyShareGroup: KeyShareGroup = keyShare[1];
  const { label, threshold, parties, items } = keyShareGroup;

  if (items.length === 0) {
    throw new Error("Invalid key share, no items found");
  }

  const onShareChange = (n: number) => {
    setSelectedParty(n);
  };

  const onTransaction = async (signValue: SignTransaction) => {
    const formData: GroupFormData = [label, { parties, threshold }];

    try {
      // Create the remote server group and session and store
      // the information in the redux state before proceeding to
      // the next view
      await createGroupSession(
        SessionKind.SIGN,
        formData,
        websocket,
        dispatch,
        selectedParty || items[0],
        signValue
      );

      // Store the sign candidate state
      const signCandidate = {
        address,
        selectedParty: selectedParty || items[0],
        value: signValue,
        signingType: SigningType.TRANSACTION,
        creator: true,
      };
      dispatch(setSignCandidate(signCandidate));

      handleNext();
    } catch (e) {
      console.error(e);
      dispatch(
        setSnackbar({
          message: e.message || "",
          severity: "error",
        })
      );
    }
  };

  const stepProps = {
    next: handleNext,
    keyShare: keyShareGroup,
    selectedParty: selectedParty || items[0],
    onShareChange,
    onTransaction,
  };

  const selectedKeyShare =
    signProof === null ? (
      <>
        <Box sx={{ flexGrow: 1 }} />
        <Chip
          label={`Using key share for party #${selectedParty || items[0]}`}
        />
      </>
    ) : null;

  return (
    <>
      <Stack spacing={2}>
        <Stack spacing={1}>
          <Breadcrumbs aria-label="breadcrumb">
            <Link underline="hover" color="inherit" href="#/keys">
              Keys
            </Link>
            <Link underline="hover" color="inherit" href={"#/keys/" + address}>
              {label}
            </Link>
            <Typography color="text.primary">Sign Transaction</Typography>
          </Breadcrumbs>
          <Typography variant="h3" component="div">
            {label}
          </Typography>
        </Stack>
        <Stack direction="row" alignItems="center">
          <PublicAddress address={address} abbreviate />
          {selectedKeyShare}
        </Stack>
        <SignStepper steps={steps} activeStep={activeStep} />
        {getStepComponent(activeStep, stepProps)}
        <Signer />
        <ListenerCleanup />
      </Stack>
    </>
  );
}
