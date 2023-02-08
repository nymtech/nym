import React from 'react';
import { Checkbox, FormControlLabel, Stack, TextField, Typography } from '@mui/material';
import { Warning } from './Warning';

export const Mnemonic = ({
  mnemonic,
  confirmed,
  handleConfirmed,
}: {
  mnemonic: string;
  confirmed?: boolean;
  handleConfirmed?: (confirmed: boolean) => void;
}) => (
  <Stack spacing={2}>
    <Warning>
      <Typography sx={{ textAlign: 'center' }}>
        Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future
      </Typography>
    </Warning>
    <TextField
      label="Mnemonic"
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

    {handleConfirmed && (
      <FormControlLabel
        label="I saved my mnemonic"
        control={<Checkbox checked={confirmed} onChange={(_, checked) => handleConfirmed(checked)} />}
      />
    )}
  </Stack>
);
