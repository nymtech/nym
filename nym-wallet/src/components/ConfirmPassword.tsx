import React, { useState } from 'react';
import { Button, CircularProgress, DialogActions, DialogContent, Typography } from '@mui/material';
import { PasswordInput } from './textfields';

export const ConfirmPassword = ({
  error,
  isLoading,
  onConfirm,
  buttonTitle,
}: {
  error?: string;
  isLoading?: boolean;
  buttonTitle: string;
  onConfirm: (password: string) => void;
}) => {
  const [value, setValue] = useState('');

  return (
    <>
      <DialogContent>
        <Typography variant="body1" sx={{ color: 'error.main', my: 2 }}>
          {error}
        </Typography>

        <PasswordInput
          password={value}
          onUpdatePassword={(pswrd) => setValue(pswrd)}
          placeholder="Confirm password"
          autoFocus
        />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0 }}>
        <Button
          disabled={!value.length || isLoading}
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => onConfirm(value)}
          endIcon={isLoading && <CircularProgress size={20} />}
        >
          {buttonTitle}
        </Button>
      </DialogActions>
    </>
  );
};
