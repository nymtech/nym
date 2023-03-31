import React, { useEffect, useState } from 'react';
import { Stack, Typography } from '@mui/material';
import { AuthProvider } from '../../context';
import { isPasswordCreated } from '../../requests';
import { CreatePasswordForm, UpdatePasswordForm } from '../../components/Settings';

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
      <Stack direction="row" justifyContent="space-between" padding={3}>
        <Stack direction="column" gap={1}>
          {passwordExists ? (
            <Typography variant="h6">Change password</Typography>
          ) : (
            <Typography variant="h6">Create a password</Typography>
          )}
          <Typography variant="caption" sx={{ color: 'nym.text.muted', maxWidth: '220px' }}>
            Create strong password, min 8 characters, at least one capital letter, number and special character
          </Typography>
        </Stack>
        {passwordExists && <UpdatePasswordForm />}
        {!passwordExists && <CreatePasswordForm />}
      </Stack>
    </AuthProvider>
  );
};

export default SecuritySettings;
