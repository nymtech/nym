import React, { useState } from 'react';
import { Box, IconButton, Link, Stack, TextField } from '@mui/material';
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
        label="Mnemonic"
        type={showPassword ? 'input' : 'password'}
        value={mnemonic}
        onChange={(e) => onUpdateMnemonic(e.target.value)}
        multiline={!!showPassword}
        rows={4}
        autoFocus
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
  showForgottenPassword?: boolean;
  autoFocus?: boolean;
  onUpdatePassword: (password: string) => void;
}> = ({ password, label, error, showForgottenPassword, autoFocus, onUpdatePassword }) => {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <Stack spacing={2}>
      <Box>
        <TextField
          label={label}
          fullWidth
          value={password}
          onChange={(e) => onUpdatePassword(e.target.value)}
          type={showPassword ? 'input' : 'password'}
          autoFocus={autoFocus}
          InputProps={{
            endAdornment: (
              <IconButton onClick={() => setShowPassword((show) => !show)}>
                {showPassword ? <VisibilityOff /> : <Visibility />}
              </IconButton>
            ),
          }}
        />
        {/* currently unused */}
        {showForgottenPassword && (
          <Link
            underline="none"
            variant="body2"
            component="div"
            sx={{ mt: 1, textAlign: 'right', color: 'info.main', cursor: 'pointer' }}
            href="/forgotten-password"
          >
            Forgotten password?
          </Link>
        )}
      </Box>
      {error && <Error message={error} />}
    </Stack>
  );
};
