import React, { useState } from 'react';
import { Checkbox, FormControlLabel, Stack, TextField, Typography } from '@mui/material';
import { TopLogoLayout } from 'src/layouts/TopLogo';

import { Button } from 'src/components/Button';
import { useNavigate } from 'react-router-dom';

export const Register = () => {
  const [mnemonic, setMnemonic] = useState();
  const [isConfirmed, setIsconfirmed] = useState(false);
  const navigate = useNavigate();

  return (
    <TopLogoLayout
      title="Mnemonic"
      description="Write down your mnemonic"
      Actions={
        <Button
          fullWidth
          variant="contained"
          size="large"
          disabled={!isConfirmed}
          onClick={() => navigate('/register/create-password')}
        >
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

        <FormControlLabel
          label="I saved my mnemonic"
          control={<Checkbox checked={isConfirmed} onChange={(_, checked) => setIsconfirmed(checked)} />}
        />
      </Stack>
    </TopLogoLayout>
  );
};
