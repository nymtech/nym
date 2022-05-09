import React from 'react';
import { Alert, Button, Stack, TextField, Typography } from '@mui/material';
import { Check, ContentCopySharp } from '@mui/icons-material';

export const Mnemonic = ({
  mnemonic,
  copied,
  handleCopy,
}: {
  mnemonic: string;
  copied: boolean;
  handleCopy: (text?: string) => void;
}) => (
  <Stack spacing={2} alignItems="center">
    <Alert severity="warning" icon={false} sx={{ display: 'block' }}>
      <Typography sx={{ textAlign: 'center' }}>
        Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future
      </Typography>
    </Alert>
    <TextField multiline rows={3} value={mnemonic} fullWidth />

    <Button
      color="inherit"
      disableElevation
      size="large"
      onClick={() => {
        handleCopy(mnemonic);
      }}
      sx={{
        width: 250,
      }}
      endIcon={!copied ? <ContentCopySharp /> : <Check color="success" />}
    >
      Copy mnemonic
    </Button>
  </Stack>
);
