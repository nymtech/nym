/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { Button, Stack } from '@mui/material';
import { SubtitleSlick, Title } from '../components';
import { TPages } from '../types';

export const WelcomeContent: React.FC<{
  page: TPages;
  onUseExisting: () => void;
  onCreateAccount: () => void;
}> = ({ onUseExisting, onCreateAccount }) => (
  <>
    <Title title="Welcome to NYM" />
    <SubtitleSlick subtitle="Next generation of privacy" />
    <Stack spacing={3} sx={{ width: 300 }}>
      <Button fullWidth color="primary" variant="contained" size="large" onClick={onUseExisting}>
        Sign in
      </Button>
      <Button fullWidth color="inherit" disableElevation size="large" onClick={onCreateAccount}>
        Create account
      </Button>
    </Stack>
  </>
);
