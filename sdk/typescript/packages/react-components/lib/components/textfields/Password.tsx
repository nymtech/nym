import { useState } from 'react';
import { Box, IconButton, Stack, TextField } from '@mui/material';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import { Error } from '../warnings/Error';

export type PasswordInputProps = {
  password: string;
  error?: string;
  label?: string;
  placeholder?: string;
  autoFocus?: boolean;
  disabled?: boolean;
  onUpdatePassword: (password: string) => void;
};

export const PasswordInput = ({
  password,
  label,
  placeholder,
  error,
  autoFocus,
  disabled,
  onUpdatePassword,
}: PasswordInputProps) => {
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
          InputLabelProps={{ shrink: true }}
        />
      </Box>
      {error && <Error message={error} />}
    </Stack>
  );
};
