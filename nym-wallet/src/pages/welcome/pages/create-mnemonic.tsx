import React, { useContext, useEffect, useState } from 'react';
import { Alert, Button, CircularProgress, Grid, ToggleButton, ToggleButtonGroup, Typography } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react';
import { useSnackbar } from 'notistack';
import { WordTiles } from '../components';
import { TPages } from '../types';
import { MnemonicInput } from '../components/textfields';
import { SignInContext } from '../context';
import { createPassword } from '../../../requests';

export const CreateMnemonic = ({
  page,
  onNext,
  onPrev,
  onComplete,
}: {
  page: TPages;
  onNext: () => void;
  onPrev: () => void;
  onComplete: () => void;
}) => {
  const [toggle, setToggle] = useState('new');
  const [isLoading, setIsLoading] = useState(false);

  const { password, mnemonic, mnemonicWords, error, generateMnemonic, validateMnemonic, setMnemonic, setError } =
    useContext(SignInContext);

  const { enqueueSnackbar } = useSnackbar();

  const handleUseExisting = async () => {
    setIsLoading(true);
    setError(undefined);
    try {
      await validateMnemonic();
      await createPassword({ password, mnemonic });
      enqueueSnackbar('Password successfully created', { variant: 'success' });
      onComplete();
    } catch (e) {
      setError(e as string);
      setIsLoading(false);
    }
  };

  useEffect(() => {
    setError(undefined);
    if (toggle === 'new') {
      generateMnemonic();
    } else {
      setMnemonic('');
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
          onChange={(_: React.MouseEvent<HTMLElement>, value: string) => {
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
          <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(mnc) => setMnemonic(mnc)} error={error} />
        )}
      </Grid>

      <Grid container item spacing={2} justifyContent="center">
        <Grid item>
          <Button
            variant="outlined"
            disableElevation
            size="large"
            onClick={() => {
              setError(undefined);
              onPrev();
            }}
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
            onClick={toggle === 'new' ? onNext : handleUseExisting}
            sx={{ width: 250 }}
          >
            {isLoading ? <CircularProgress size={20} color="inherit" /> : 'Verify mnemonic'}
          </Button>
        </Grid>
      </Grid>
    </Grid>
  );
};
