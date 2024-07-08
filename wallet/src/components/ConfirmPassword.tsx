import React, { useEffect, useState } from 'react';
import { Button, CircularProgress, DialogActions, DialogContent, Typography } from '@mui/material';
import { useKeyPress } from '@src/hooks/useKeyPress';
import { PasswordInput } from '@nymproject/react';
import { StyledBackButton } from './StyledBackButton';

export const ConfirmPassword = ({
  error,
  isLoading,
  onConfirm,
  onCancel,
  buttonTitle,
}: {
  error?: string;
  isLoading?: boolean;
  buttonTitle: string;
  onCancel?: () => void;
  onConfirm: (password: string) => void;
}) => {
  const [value, setValue] = useState('');

  const enterKeyPressed = useKeyPress('Enter');

  useEffect(() => {
    if (enterKeyPressed && !!value.length && !isLoading) {
      onConfirm(value);
    }
  }, [enterKeyPressed]);

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
          disabled={isLoading}
        />
      </DialogContent>
      <DialogActions sx={{ p: 3, pt: 0, gap: 2 }}>
        {onCancel && <StyledBackButton onBack={onCancel} />}
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
