import React from 'react';
import { Alert, Button, Typography } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react';
import { WordTiles } from '../components';
import { TMnemonicWords, TPages } from '../types';

export const MnemonicWords = ({
  mnemonicWords,
  mnemonic,
  onNext,
  onPrev,
  page,
}: {
  mnemonicWords?: TMnemonicWords;
  mnemonic?: string;
  onNext: () => void;
  onPrev: () => void;
  page: TPages;
}) => (
  <>
    <div id={page} />
    <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Write down your mnemonic</Typography>
    <Alert
      icon={false}
      severity="info"
      sx={{ bgcolor: '#18263B', color: '#50ABFF', width: 625 }}
      action={mnemonic && <CopyToClipboard value={mnemonic} tooltip="Copy your mnemonic phrase" />}
    >
      Please store your mnemonic in a safe place. This is the only way to access your wallet!
    </Alert>
    <WordTiles mnemonicWords={mnemonicWords} showIndex />
    <Button variant="contained" color="primary" disableElevation size="large" onClick={onNext} sx={{ width: 250 }}>
      Verify mnemonic
    </Button>
    <Button
      variant="outlined"
      disableElevation
      size="large"
      onClick={onPrev}
      sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' }, width: 250 }}
    >
      Back
    </Button>
  </>
);
