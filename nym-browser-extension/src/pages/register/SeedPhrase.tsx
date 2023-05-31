import React, { useRef, useState } from 'react';
import { Checkbox, FormControlLabel, Stack, TextField, Typography } from '@mui/material';
import { TopLogoLayout } from 'src/layouts/TopLogo';
import { Button } from 'src/components/ui';
import { useNavigate } from 'react-router-dom';
import { generateMnemonmic } from 'src/validator-client';
import { useRegisterContext } from 'src/context/register';

export const SeedPhrase = () => {
  const [isConfirmed, setIsconfirmed] = useState(false);
  const navigate = useNavigate();

  const seedPhrase = useRef(generateMnemonmic());

  const { createAccount, userPassword } = useRegisterContext();

  const handleEncryptSeedPhrase = async () => {
    await createAccount({mnemonic: seedPhrase.current, password: userPassword, accountName: "Default account"});
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
