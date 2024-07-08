import React from 'react';
import { Box, Checkbox, FormControlLabel, Stack, TextField, Typography } from '@mui/material';
import { Title } from '@src/pages/auth/components/heading';
import { Warning } from './Warning';

export const Mnemonic = ({
  mnemonic,
  confirmed,
  withTitle,
  handleConfirmed,
}: {
  mnemonic: string;
  confirmed?: boolean;
  withTitle?: boolean;
  handleConfirmed?: (confirmed: boolean) => void;
}) => (
  <Stack spacing={2}>
    {withTitle && (
      <Box sx={{ pb: 2, textAlign: 'center' }}>
        <Title title="Copy and save or write down your mnemonic" />
      </Box>
    )}
    <Box sx={{ pb: 2 }}>
      <Warning>
        <Typography sx={{ textAlign: 'center' }}>
          Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future
        </Typography>
      </Warning>
    </Box>
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
