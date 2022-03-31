import React, { useContext, useState } from 'react';
import { Button, FormControl, Stack } from '@mui/material';
import { MnemonicInput, Subtitle } from '../components';
import { ClientContext } from '../../../context/main';
import { TPages } from '../types';

export const SignInMnemonic = ({ page, onPrev }: { page: TPages; onPrev: () => void }) => {
  const [mnemonic, setMnemonic] = useState('');
  const { setError, logIn, error } = useContext(ClientContext);

  return (
    <Stack spacing={2} alignItems="center" minWidth="50%">
      <Subtitle subtitle="Enter a mnemonic to sign in" />
      <FormControl fullWidth>
        <Stack spacing={2}>
          <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(mnc) => setMnemonic(mnc)} error={error} />

          <Button
            variant="contained"
            size="large"
            fullWidth
            onClick={() => logIn({ type: 'mnemonic', value: mnemonic })}
          >
            {`Sign in with mnemonic`}
          </Button>
          <Button
            variant="outlined"
            disableElevation
            size="large"
            onClick={() => {
              setError(undefined);
              onPrev();
            }}
            fullWidth
            sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
          >
            Back
          </Button>
        </Stack>
      </FormControl>
    </Stack>
  );
};
