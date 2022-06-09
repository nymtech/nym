import React, { useContext, useEffect } from 'react';
import { useHistory } from 'react-router-dom';
import { Button, Stack, Typography } from '@mui/material';
import { AuthContext } from 'src/context/auth';
import { Check, ContentCopySharp } from '@mui/icons-material';
import { useClipboard } from 'use-clipboard-copy';
import { WordTiles } from '../components';
import { Warning } from '../../../components';

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
    <Stack alignItems="center" spacing={3}>
      <Typography sx={{ color: 'common.white', fontWeight: 600 }} variant="h4" textAlign="center">
        Copy and save or write down your mnemonic
      </Typography>

      <Warning>
        <Typography>
          Below is your 24 word mnemonic, make sure to store it in a safe place for accessing your wallet in the future.
        </Typography>
      </Warning>

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
  );
};
