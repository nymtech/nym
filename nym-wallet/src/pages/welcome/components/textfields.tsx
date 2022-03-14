import React, { useContext, useState } from 'react';
import { Alert, Button, IconButton, Stack, TextField } from '@mui/material';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import { ClientContext } from '../../../context/main';

export const MnemonicInput = () => {
  const [mnemonic, setMnemonic] = useState('');
  const { logIn, error } = useContext(ClientContext);
  return (
    <Stack spacing={2}>
      <TextField
        placeholder="Mnemonic"
        value={mnemonic}
        onChange={(e) => setMnemonic(e.target.value)}
        multiline
        rows={5}
        fullWidth
      />
      {error && (
        <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light', width: '100%' }}>
          {error}
        </Alert>
      )}
      <Button variant="contained" size="large" fullWidth onClick={() => logIn(mnemonic)}>
        Sign in with mnemonic
      </Button>
    </Stack>
  );
};

export const PasswordInput = () => {
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const { signInWithPassword, error } = useContext(ClientContext);

  return (
    <Stack spacing={2}>
      <TextField
        label="Password"
        fullWidth
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        type={showPassword ? 'input' : 'password'}
        InputProps={{
          endAdornment: (
            <IconButton onClick={() => setShowPassword((show) => !show)}>
              {showPassword ? <VisibilityOff /> : <Visibility />}
            </IconButton>
          ),
        }}
      />
      {error && (
        <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light', width: '100%' }}>
          {error}
        </Alert>
      )}
      <Button variant="contained" size="large" fullWidth onClick={() => signInWithPassword(password)}>
        Sign in with password
      </Button>
    </Stack>
  );
};
