import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Container, Button, Stack } from '@mui/material';
import { AuthContext } from '@src/context/auth';
import { Mnemonic } from '../../../components';

export const CreateMnemonic = () => {
  const { mnemonic, mnemonicWords, generateMnemonic, resetState } = useContext(AuthContext);
  const navigate = useNavigate();
  const [confirmed, setConfirmed] = useState(false);

  useEffect(() => {
    if (mnemonicWords.length === 0) {
      generateMnemonic();
    }
  }, []);

  return (
    <Container maxWidth="sm">
      <Stack alignItems="center" spacing={3} maxWidth="xs">
        <Mnemonic mnemonic={mnemonic} handleConfirmed={setConfirmed} confirmed={confirmed} withTitle />
        <Button
          variant="contained"
          color="primary"
          disableElevation
          size="large"
          onClick={() => navigate('/verify-mnemonic')}
          sx={{ width: '100%', fontSize: 15 }}
          disabled={!confirmed}
        >
          Continue
        </Button>
        <Button
          onClick={() => {
            resetState();
            navigate(-1);
          }}
          color="inherit"
        >
          Back
        </Button>
      </Stack>
    </Container>
  );
};
