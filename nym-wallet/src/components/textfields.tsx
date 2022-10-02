import React, { useState } from 'react';
import { Box, IconButton, Stack, TextField } from '@mui/material';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import { Error } from './Error';

export const MnemonicInput: React.FC<{
  mnemonic: string;
  error?: string;
  onUpdateMnemonic: (mnemonic: string) => void;
}> = ({ mnemonic, error, onUpdateMnemonic }) => {
  const [showPassword, setShowPassword] = useState(false);
  return (
    <Stack spacing={2}>
      <TextField
        label="Mnemonic"
        type="input"
        value={showPassword ? mnemonic : mnemonic.replaceAll(/./g, '*')}
        onChange={(e) => onUpdateMnemonic(e.target.value)}
        multiline
        autoFocus
        fullWidth
        inputProps={{
          style: {
            height: '160px',
          },
        }}
        sx={{
          'input::-webkit-textfield-decoration-container': {
            alignItems: 'start',
          },
        }}
      />
      <FormControlLabel
        control={<Checkbox checked={Boolean(showPassword)} onChange={() => setShowPassword((show) => !show)} />}
        label="Reveal my mnemonic"
      />
      {error && <Error message={error} />}
    </Stack>
  );
};

export const PasswordInput: React.FC<{
  password: string;
  error?: string;
  label?: string;
  placeholder?: string;
  autoFocus?: boolean;
  disabled?: boolean;
  onUpdatePassword: (password: string) => void;
}> = ({ password, label, placeholder, error, autoFocus, disabled, onUpdatePassword }) => {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <Stack spacing={2}>
      <Box>
        <TextField
          label={label}
          placeholder={placeholder}
          fullWidth
          value={password}
          onChange={(e) => onUpdatePassword(e.target.value)}
          type={showPassword ? 'input' : 'password'}
          autoFocus={autoFocus}
          disabled={disabled}
          InputProps={{
            endAdornment: (
              <IconButton onClick={() => setShowPassword((show) => !show)}>
                {showPassword ? <VisibilityOff /> : <Visibility />}
              </IconButton>
            ),
          }}
        />
      </Box>
      {error && <Error message={error} />}
    </Stack>
  );
};
