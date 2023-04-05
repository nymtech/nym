import { Stack, TextField } from '@mui/material';
import React from 'react';
import { Button } from 'src/components/Button';
import { CenteredLogoLayout } from 'src/layouts/CenteredLogo';

export const Login = () => (
  <CenteredLogoLayout
    title="Privacy crypto wallet"
    Actions={
      <Stack gap={1} width="100%" justifyContent="flex-end">
        <TextField placeholder="Password" sx={{ mb: 3 }} />
        <Button variant="contained" disableElevation size="large" fullWidth>
          Unlock
        </Button>
        <Button variant="outlined" disableElevation size="large" fullWidth color="primary">
          Forgot password?
        </Button>
      </Stack>
    }
  />
);
