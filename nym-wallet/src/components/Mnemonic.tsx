import React from 'react';
import { Button, Stack, TextField, Typography } from '@mui/material';
import { Check, ContentCopySharp } from '@mui/icons-material';
import { Warning } from './Warning';
import { MnemonicModal } from './Accounts/modals/MnemonicModal';

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
    <Warning>
      <Typography sx={{ textAlign: 'center' }} data-testid="below24word">
        Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future
      </Typography>
    </Warning>
    <TextField
      label="Mnemonic"
      id="mnemonicPhrase"
      type="input"
      value={mnemonic}
      multiline
      autoFocus={false}
      fullWidth
      inputProps={{ 
        style: {
          height: '160px',
        },
      }}
      InputLabelProps={{ shrink: true }}
      sx={{
        'input::-webkit-textfield-decoration-container': {
          alignItems: 'start',
        },
      }}
    />

    <Button
      data-testid="copyMnemonic"
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
