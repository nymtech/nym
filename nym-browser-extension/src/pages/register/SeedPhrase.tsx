import React, { useRef, useState } from 'react';
import { Checkbox, FormControlLabel, Stack, TextField, Typography } from '@mui/material';
import { TopLogoLayout } from 'src/layouts/TopLogo';
import { Button } from 'src/components/ui';
import { useLocation, useNavigate } from 'react-router-dom';
import { generateMnemonmic } from 'src/validator-client';
import { encrypt } from 'src/utils/crypto';

export const SeedPhrase = () => {
  const [isConfirmed, setIsconfirmed] = useState(false);
  const navigate = useNavigate();
  const { state } = useLocation();

  const seedPhrase = useRef(generateMnemonmic());

  const handleEncryptSeedPhrase = () => {
    // encrypt(state.password, seedPhrase.current);
    navigate('/register/complete');
  };

  return (
    <TopLogoLayout
      title="Seed phrase"
      description="Save your seed phrase"
      Actions={
        <Button fullWidth variant="contained" size="large" disabled={!isConfirmed} onClick={handleEncryptSeedPhrase}>
          Next
        </Button>
      }
    >
      <Stack spacing={2} sx={{ mt: 2 }}>
        <Typography sx={{ textAlign: 'center', color: 'error.dark' }}>
          Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future
        </Typography>

        <TextField
          label="Mnemonic"
          type="input"
          value={seedPhrase.current}
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

        <FormControlLabel
          label="I saved my mnemonic"
          control={<Checkbox checked={isConfirmed} onChange={(_, checked) => setIsconfirmed(checked)} />}
        />
      </Stack>
    </TopLogoLayout>
  );
};
