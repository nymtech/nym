import React from 'react';
import Typography from '@mui/material/Typography';
import WarningAmberIcon from '@mui/icons-material/WarningAmber';

export const TermsAndConditions: React.FC<{
  error?: boolean;
}> = ({ error }) => (
  <Typography display="inline" color={(theme) => (error ? theme.palette.error.main : undefined)}>
    I agree to the{' '}
    <a href="https://nymtech.net/terms-and-conditions/operators/v1.0.0" target="_blank" rel="noreferrer">
      operator terms and conditions
    </a>
  </Typography>
);

export const TermsAndConditionsHelp: React.FC<{
  error?: boolean;
  helperText?: string;
}> = ({ error, helperText }) => {
  if (!error || !helperText) {
    return null;
  }
  return (
    <Typography color="error.main" display="flex" alignItems="center">
      <WarningAmberIcon sx={{ mr: 1 }} />
      {helperText}
    </Typography>
  );
};
