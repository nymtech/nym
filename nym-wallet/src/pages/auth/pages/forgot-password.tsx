/* eslint-disable react/no-unused-prop-types */
import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Stack, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import { archiveWalletFile } from 'src/requests';
import { Console } from 'src/utils/console';
import { Subtitle } from '../components';

export const ForgotPassword = () => {
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();
  return (
    <>
      <Subtitle subtitle="Create a new password or sign in with mnemonic" />
      <Stack spacing={2} sx={{ width: 300 }}>
        <Button
          variant="contained"
          size="large"
          onClick={async () => {
            try {
              await archiveWalletFile();
            } catch (e) {
              Console.error(e);
              enqueueSnackbar('Failed to archive wallet file', { variant: 'error' });
            }
            navigate('/confirm-mnemonic');
          }}
          fullWidth
        >
          Create a new password
        </Button>
        <Typography sx={{ textAlign: 'center', fontWeight: 600 }}>or</Typography>
        <Button size="large" variant="contained" fullWidth onClick={() => navigate('/sign-in-mnemonic')}>
          Sign in with mnemonic
        </Button>
        <Button color="inherit" onClick={() => navigate(-1)}>
          Back
        </Button>
      </Stack>
    </>
  );
};
