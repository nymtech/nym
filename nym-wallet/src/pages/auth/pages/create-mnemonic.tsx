import React, { useContext, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Alert, Button, Stack, Typography } from '@mui/material';
import { AuthContext } from 'src/context/auth';
import { Check, ContentCopySharp } from '@mui/icons-material';
import { useClipboard } from 'use-clipboard-copy';
import { WordTiles } from '../components';

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
    <Stack alignItems="center" spacing={3}>
      <Typography sx={{ color: 'common.white', fontWeight: 600 }} textAlign="center">
        Write down your mnemonic
      </Typography>

      <Alert variant="outlined" severity="warning" sx={{ textAlign: 'center' }}>
        <Typography>
          Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future.
        </Typography>
      </Alert>

      <WordTiles mnemonicWords={mnemonicWords} showIndex />

      <Button
        color="inherit"
        disableElevation
        size="large"
        onClick={() => {
          copy(mnemonic);
        }}
        sx={{
          width: 250,
        }}
        endIcon={!copied ? <ContentCopySharp /> : <Check color="success" />}
      >
        Copy mnemonic
      </Button>

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
  );
};
