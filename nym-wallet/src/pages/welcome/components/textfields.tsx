import React, { useState } from 'react';
import { IconButton, Stack, TextField } from '@mui/material';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import { Error } from './error';

export const MnemonicInput: React.FC<{
  mnemonic: string;
  error?: string;
  onUpdateMnemonic: (mnemonic: string) => void;
}> = ({ mnemonic, error, onUpdateMnemonic }) => {
  const [showPassword, setShowPassword] = useState(false);
  return (
    <Stack spacing={2}>
      <TextField
        placeholder="Mnemonic"
        type={showPassword ? 'input' : 'password'}
        value={mnemonic}
        onChange={(e) => onUpdateMnemonic(e.target.value)}
        multiline={!!showPassword}
        rows={4}
        fullWidth
        InputProps={{
          endAdornment: (
            <IconButton onClick={() => setShowPassword((show) => !show)}>
              {showPassword ? <VisibilityOff /> : <Visibility />}
            </IconButton>
          ),
        }}
      />
      {error && <Error message={error} />}
    </Stack>
  );
};

export const PasswordInput: React.FC<{
  password: string;
  error?: string;
  label: string;
  onUpdatePassword: (password: string) => void;
}> = ({ password, label, error, onUpdatePassword }) => {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <Stack spacing={2}>
      <TextField
        label={label}
        fullWidth
        value={password}
        onChange={(e) => onUpdatePassword(e.target.value)}
        type={showPassword ? 'input' : 'password'}
        InputProps={{
          endAdornment: (
            <IconButton onClick={() => setShowPassword((show) => !show)}>
              {showPassword ? <VisibilityOff /> : <Visibility />}
            </IconButton>
          ),
        }}
      />
      {error && <Error message={error} />}
    </Stack>
  );
};
