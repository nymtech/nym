/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { Button, Stack } from '@mui/material';
import { useNavigate } from 'react-router-dom';
import { SubtitleSlick } from '../components';

export const WelcomeContent: FCWithChildren<{}> = () => {
  const navigate = useNavigate();

  return (
    <Stack spacing={1.5} sx={{ width: '100%', alignItems: 'stretch' }}>
      <SubtitleSlick subtitle="Next generation of privacy" />
      <Stack spacing={2} sx={{ width: '100%', pt: 0.5 }}>
        <Button
          fullWidth
          color="primary"
          sx={{ color: 'primary.contrastText', fontSize: 16, py: 1.25 }}
          variant="contained"
          size="large"
          onClick={() => navigate('/existing-account')}
        >
          Sign in
        </Button>
        <Button
          fullWidth
          color="inherit"
          sx={{ fontSize: 16, border: '1px solid', borderColor: 'divider' }}
          variant="outlined"
          size="large"
          onClick={() => navigate('/create-mnemonic')}
        >
          Create account
        </Button>
      </Stack>
    </Stack>
  );
};
