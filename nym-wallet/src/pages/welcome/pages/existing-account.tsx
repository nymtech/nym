/* eslint-disable react/no-unused-prop-types */
import React, { useContext, useState } from 'react';
import { Alert, Button, FormControl, Grid, Stack, ToggleButton, ToggleButtonGroup } from '@mui/material';
import { ClientContext } from 'src/context/main';
import { Subtitle } from '../components';
import { TPages } from '../types';
import { MnemonicInput, PasswordInput } from '../components/textfields';

export const ExistingAccount: React.FC<{ page: TPages; onPrev: () => void; onCreatePassword: () => void }> = ({
  onPrev,
  onCreatePassword,
}) => {
  const [toggle, setToggle] = useState('mnemonic');
  const [password, setPassword] = useState('');
  const [mnemonic, setMnemonic] = useState('');
  const { setError, logIn } = useContext(ClientContext);

  return (
    <>
      <Subtitle subtitle={`Enter your ${toggle} from existing wallet`} />
      {toggle === 'mnemonic' && (
        <Alert
          icon={false}
          sx={{ bgcolor: '#18263B', color: '#50ABFF' }}
          action={
            <Button variant="outlined" color="info" size="small" onClick={onCreatePassword}>
              Create password
            </Button>
          }
        >
          Have a mnemonic and want to log in using a password?
        </Alert>
      )}
      <Grid container justifyContent="center">
        <Grid item xs={6}>
          <Stack spacing={2}>
            <ToggleButtonGroup
              fullWidth
              value={toggle}
              exclusive
              onChange={(e: React.MouseEvent<HTMLElement>, value: string) => {
                setError(undefined);
                setToggle(value);
              }}
            >
              <ToggleButton value="mnemonic">Mnemonic</ToggleButton>
              <ToggleButton value="password">Password</ToggleButton>
            </ToggleButtonGroup>
            <FormControl fullWidth>
              <Stack spacing={2}>
                {toggle === 'mnemonic' && (
                  <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(mnc) => setMnemonic(mnc)} />
                )}
                {toggle === 'password' && (
                  <PasswordInput password={password} onUpdatePassword={(pswd) => setPassword(pswd)} />
                )}

                <Button variant="contained" size="large" fullWidth onClick={() => logIn(mnemonic)}>
                  {`Sign in with ${toggle}`}
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
        </Grid>
      </Grid>
    </>
  );
};
