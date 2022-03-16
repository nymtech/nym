import React, { useContext, useEffect, useState } from 'react';
import { Alert, Button, Grid, ToggleButton, ToggleButtonGroup, Typography } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react';
import { WordTiles } from '../components';
import { TMnemonicWords, TPages } from '../types';
import { MnemonicInput } from '../components/textfields';
import { signInWithMnemonic } from 'src/requests';
import { ClientContext } from 'src/context/main';

export const CreateAccount = ({
  mnemonicWords,
  mnemonic,
  page,
  onUseNew,
  onNext,
  onPrev,
}: {
  mnemonicWords?: TMnemonicWords;
  mnemonic: string;
  onUseNew: () => void;
  onNext: () => void;
  onPrev: () => void;
  page: TPages;
}) => {
  const [toggle, setToggle] = useState('new');
  const [existingMnemonic, setExistingMnemonic] = useState('');

  const { setError } = useContext(ClientContext);
  const validateMnemonic = async () => {
    try {
      await signInWithMnemonic(existingMnemonic);
    } catch (e) {
      setError(e as string);
    }
  };

  useEffect(() => {
    if (toggle === 'new') {
      onUseNew();
    } else {
      setExistingMnemonic('');
    }
  }, [toggle]);

  return (
    <Grid container spacing={4} justifyContent="center" id={page}>
      <Grid item xs={12}>
        <Typography sx={{ color: 'common.white', fontWeight: 600 }} textAlign="center">
          Write down your mnemonic
        </Typography>
      </Grid>
      {toggle === 'new' && (
        <Grid item xs={7}>
          <Alert
            icon={false}
            sx={{ bgcolor: '#18263B', color: '#50ABFF' }}
            action={mnemonic && <CopyToClipboard value={mnemonic} tooltip="Copy your mnemonic phrase" />}
          >
            <Typography>Please store your mnemonic in a safe place</Typography>
            <Typography fontWeight={600} textTransform="uppercase">
              This is the only way to access your wallet!
            </Typography>
          </Alert>
        </Grid>
      )}
      <Grid item xs={7}>
        <ToggleButtonGroup
          fullWidth
          value={toggle}
          exclusive
          onChange={(e: React.MouseEvent<HTMLElement>, value: string) => {
            setToggle(value);
          }}
        >
          <ToggleButton value="new">Create new mnemonic</ToggleButton>
          <ToggleButton value="existing">Use existing mnemonic</ToggleButton>
        </ToggleButtonGroup>
      </Grid>
      <Grid item xs={toggle === 'new' ? 12 : 7}>
        {toggle === 'new' && <WordTiles mnemonicWords={mnemonicWords} showIndex />}
        {toggle === 'existing' && (
          <MnemonicInput mnemonic={existingMnemonic} onUpdateMnemonic={(mnc) => setExistingMnemonic(mnc)} />
        )}
      </Grid>

      <Grid container item spacing={2} justifyContent="center">
        <Grid item>
          <Button
            variant="outlined"
            disableElevation
            size="large"
            onClick={onPrev}
            sx={{
              color: 'common.white',
              border: '1px solid white',
              '&:hover': { border: '1px solid white' },
              width: 250,
            }}
          >
            Back
          </Button>
        </Grid>
        <Grid item>
          <Button
            variant="contained"
            color="primary"
            disableElevation
            size="large"
            onClick={toggle === 'new' ? onNext : validateMnemonic}
            sx={{ width: 250 }}
          >
            Verify mnemonic
          </Button>
        </Grid>
      </Grid>
    </Grid>
  );
};
