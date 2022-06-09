import React, { useContext, useEffect } from 'react';
import { useHistory } from 'react-router-dom';
import { Button, Container, Stack } from '@mui/material';
import { AuthContext } from 'src/context/auth';
import { useClipboard } from 'use-clipboard-copy';
import { Mnemonic } from '../../../components';

export const CreateMnemonic = () => {
  const { mnemonic, mnemonicWords, generateMnemonic, resetState } = useContext(AuthContext);
  const history = useHistory();

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
          onClick={() => history.push('/verify-mnemonic')}
          sx={{ width: 250 }}
          disabled={!copied}
        >
          I saved my mnemonic
        </Button>
        <Button
          onClick={() => {
            resetState();
            history.goBack();
          }}
          color="inherit"
        >
          Back
        </Button>
      </Stack>
    </Container>
  );
};
