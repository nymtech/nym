import React, { useEffect, useState } from 'react';
import { Grid, Stack, Typography } from '@mui/material';
import { AuthProvider } from '../../context';
import { isPasswordCreated } from '../../requests';
import { PasswordForm } from '../../components/Settings';

const SecuritySettings = () => {
  const [passwordExists, setPasswordExists] = useState(false);

  const checkForPassword = async () => {
    const hasPassword = await isPasswordCreated();
    setPasswordExists(hasPassword);
  };

  useEffect(() => {
    checkForPassword();
  }, []);

  return (
    <AuthProvider>
      <Grid container spacing={2} padding={3}>
        <Grid item sm={12} md={6} lg={8}>
          <Stack direction="column" gap={1}>
            {passwordExists ? (
              <Typography variant="h6">Change password</Typography>
            ) : (
              <Typography variant="h6">Create new password</Typography>
            )}
            <Typography variant="caption" sx={{ color: 'nym.text.muted', maxWidth: '220px' }}>
              Create a strong password, min 8 characters, at least one capital letter, number and special character
            </Typography>
          </Stack>
        </Grid>
        <Grid item sm={12} md={6} lg={4}>
          <PasswordForm mode={passwordExists ? 'update' : 'create'} onPwdSaved={() => setPasswordExists(true)} />
        </Grid>
      </Grid>
    </AuthProvider>
  );
};

export default SecuritySettings;
