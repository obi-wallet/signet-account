import React from "react";

import {
  Alert,
  Stack,
  Box,
  Paper,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@mui/material";

import { DeleteKeyShare } from "./index";

interface ConfirmDeleteKeyShareProps {
  open: boolean;
  request: DeleteKeyShare;
  handleCancel: () => void;
  handleOk: (result: DeleteKeyShare) => void;
}

export default function ConfirmDeleteKeyShareDialog(
  props: ConfirmDeleteKeyShareProps
) {
  const { open, request, handleCancel, handleOk } = props;
  const [address, number] = request;

  return (
    <Dialog open={open} onClose={handleCancel}>
      <DialogTitle color="text.secondary">Delete Key Share</DialogTitle>
      <DialogContent>
        <Stack spacing={2}>
          <Typography variant="body1" component="div">
            Are you sure you want to permanently delete this key share?
          </Typography>

          <Paper variant="outlined">
            <Box padding={2}>
              Key share for party #{number} in {address}
            </Box>
          </Paper>

          <Alert severity="warning">
            Deletion is permanent, it cannot be undone
          </Alert>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={handleCancel}>Cancel</Button>
        <Button onClick={() => handleOk(request)} variant="contained">
          OK
        </Button>
      </DialogActions>
    </Dialog>
  );
}
