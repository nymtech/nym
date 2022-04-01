/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { Button, Stack } from '@mui/material';
import { useHistory } from 'react-router';
import { SubtitleSlick, Title } from '../components';

export const WelcomeContent: React.FC<{}> = () => {
  const history = useHistory();

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
          onClick={() => history.push('/existing-account')}
        >
          Sign in
        </Button>
        <Button
          fullWidth
          color="inherit"
          disableElevation
          size="large"
          onClick={() => history.push('/create-mnemonic')}
        >
          Create account
        </Button>
      </Stack>
    </>
  );
};
