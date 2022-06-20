/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { Button, Stack } from '@mui/material';
import { useNavigate } from 'react-router-dom';
import { SubtitleSlick, Title } from '../components';

export const WelcomeContent: React.FC<{}> = () => {
  const navigate = useNavigate();

  return (
    <>
      <Title title="Welcome to NYM" />
      <SubtitleSlick subtitle="Next generation of privacy" />
      <Stack spacing={3} minWidth={300}>
        <Button
          fullWidth
          color="primary"
          variant="contained"
          size="large"
          onClick={() => navigate('/existing-account')}
        >
          Sign in
        </Button>
        <Button fullWidth color="inherit" disableElevation size="large" onClick={() => navigate('/create-mnemonic')}>
          Create account
        </Button>
      </Stack>
    </>
  );
};
