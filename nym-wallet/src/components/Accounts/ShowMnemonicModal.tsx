import React from 'react';
import {
  Alert,
  AlertTitle,
  Box,
  Dialog,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  Typography,
} from '@mui/material';
import { Close } from '@mui/icons-material';
import { CopyToClipboard } from '@nymproject/react';

export const ShowMnemonicModal = ({
  mnemonic,
  show,
  onClose,
}: {
  mnemonic: string;
  show: boolean;
  onClose: () => void;
}) => (
  <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6">Show mnemonic</Typography>
        <IconButton onClick={onClose}>
          <Close />
        </IconButton>
      </Box>
    </DialogTitle>
    <DialogContent>
      <Stack spacing={2}>
        <Alert severity="warning" icon={false} sx={{ display: 'block' }}>
          <Typography sx={{ textAlign: 'center' }}>DO NOT share this phrase with anyone!</Typography>
          <Typography sx={{ textAlign: 'center' }}>These words can be used to steal all your accounts.</Typography>
        </Alert>
        <Alert color="info" action={<CopyToClipboard value={mnemonic} />} icon={false}>
          <AlertTitle sx={{ fontWeight: 700 }}>Mnemonic</AlertTitle>
          <Typography variant="body1">{mnemonic}</Typography>
        </Alert>
      </Stack>
    </DialogContent>
  </Dialog>
);
