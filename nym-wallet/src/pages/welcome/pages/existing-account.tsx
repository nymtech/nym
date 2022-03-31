/* eslint-disable react/no-unused-prop-types */
import React, { useContext, useState } from 'react';
import { Alert, Button, FormControl, Stack, ToggleButton, ToggleButtonGroup } from '@mui/material';
import { ClientContext } from 'src/context/main';
import { Subtitle, MnemonicInput, PasswordInput } from '../components';
import { TLoginType, TPages } from '../types';

export const ExistingAccount: React.FC<{
  page: TPages;
  loginType: TLoginType;
  setLoginType: (type: 'mnemonic' | 'password') => void;
  onPrev: () => void;
}> = ({ loginType, setLoginType, onPrev }) => {
  const [password, setPassword] = useState('');
  const [mnemonic, setMnemonic] = useState('');
  const { setError, logIn, error } = useContext(ClientContext);

  return (
    <>
      <Subtitle subtitle={`Enter your ${loginType} for existing wallet`} />
      <Alert variant="outlined" severity="info">
        You can use either a mnemonic or a password to access your wallet
      </Alert>
      <Stack spacing={2} minWidth="50%">
        <ToggleButtonGroup
          fullWidth
          value={loginType}
          exclusive
          onChange={(_: React.MouseEvent<HTMLElement>, value: TLoginType) => {
            setError(undefined);
            setLoginType(value);
          }}
        >
          <ToggleButton value="mnemonic">Mnemonic</ToggleButton>
          <ToggleButton value="password">Password</ToggleButton>
        </ToggleButtonGroup>
        <FormControl fullWidth>
          <Stack spacing={2}>
            {loginType === 'mnemonic' && (
              <MnemonicInput mnemonic={mnemonic} onUpdateMnemonic={(mnc) => setMnemonic(mnc)} error={error} />
            )}
            {loginType === 'password' && (
              <PasswordInput
                password={password}
                onUpdatePassword={(pswd) => setPassword(pswd)}
                label="Password"
                error={error}
              />
            )}

            <Button
              variant="contained"
              size="large"
              fullWidth
              onClick={() => logIn({ type: loginType, value: loginType === 'mnemonic' ? mnemonic : password })}
            >
              {`Sign in with ${loginType}`}
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
    </>
  );
};
