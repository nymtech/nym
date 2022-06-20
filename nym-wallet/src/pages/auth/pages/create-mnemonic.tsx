import React, { useContext, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Container, Button, Stack } from '@mui/material';
import { AuthContext } from 'src/context/auth';
import { useClipboard } from 'use-clipboard-copy';
import { Mnemonic } from '../../../components';

export const CreateMnemonic = () => {
  const { mnemonic, mnemonicWords, generateMnemonic, resetState } = useContext(AuthContext);
  const navigate = useNavigate();

  useEffect(() => {
    if (mnemonicWords.length === 0) {
      generateMnemonic();
    }
  }, []);

  const { copy, copied } = useClipboard({ copiedTimeout: 5000 });
  return (
    <Container maxWidth="xs">
      <Stack alignItems="center" spacing={3} maxWidth="xs">
        <Mnemonic mnemonic={mnemonic} handleCopy={copy} copied={copied} />

        <Button
          variant="contained"
          color="primary"
          disableElevation
          size="large"
          onClick={() => navigate('/verify-mnemonic')}
          sx={{ width: 250 }}
          disabled={!copied}
        >
          I saved my mnemonic
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
