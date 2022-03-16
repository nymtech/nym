import React, { useContext, useState } from 'react';
import { Alert, Button, IconButton, Stack, TextField } from '@mui/material';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import { ClientContext } from '../../../context/main';

export const MnemonicInput: React.FC<{ mnemonic: string; onUpdateMnemonic: (mnemonic: string) => void }> = ({
  mnemonic,
  onUpdateMnemonic,
}) => {
  const { error } = useContext(ClientContext);
  return (
    <Stack spacing={2}>
      <TextField
        placeholder="Mnemonic"
        value={mnemonic}
        onChange={(e) => onUpdateMnemonic(e.target.value)}
        multiline
        rows={5}
        fullWidth
      />
      {error && (
        <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light', width: '100%' }}>
          {error}
        </Alert>
      )}
    </Stack>
  );
};

export const PasswordInput: React.FC<{ password: string; onUpdatePassword: (password: string) => void }> = ({
  password,
  onUpdatePassword,
}) => {
  const [showPassword, setShowPassword] = useState(false);
  const { error } = useContext(ClientContext);

  return (
    <Stack spacing={2}>
      <TextField
        label="Password"
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
      {error && (
        <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light', width: '100%' }}>
          {error}
        </Alert>
      )}
    </Stack>
  );
};
