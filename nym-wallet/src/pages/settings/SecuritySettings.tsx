import React, { useEffect, useState } from 'react';
import { Grid, Stack, Typography } from '@mui/material';
import { isPasswordCreated } from '../../requests';
import { PasswordCreateForm, PasswordUpdateForm } from '../../components/Settings';
import { ConfirmationModal } from '../../components';

const SecuritySettings = () => {
  const [passwordExists, setPasswordExists] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);

  const checkForPassword = async () => {
    const hasPassword = await isPasswordCreated();
    setPasswordExists(hasPassword);
  };

  const onPasswordCreated = () => {
    setPasswordExists(true);
    setModalOpen(true);
  };

  useEffect(() => {
    checkForPassword();
  }, []);

  return (
    <>
      <ConfirmationModal
        title={`Password successfully ${passwordExists ? 'changed' : 'created'}`}
        onClose={() => setModalOpen(false)}
        onConfirm={() => setModalOpen(false)}
        maxWidth="xs"
        confirmButton="OK"
        open={modalOpen}
      >
        <Stack alignItems="center" spacing={2}>
          <Typography>To use all the features of the wallet now, log in to it with your new password</Typography>
        </Stack>
      </ConfirmationModal>
      <Grid container spacing={2} padding={3}>
        <Grid item sm={12} md={6} lg={8}>
          <Stack direction="column" gap={1}>
            {passwordExists ? (
              <Typography variant="h6">Change your password</Typography>
            ) : (
              <Typography variant="h6">Create a password</Typography>
            )}

            {passwordExists ? (
              <Typography variant="caption" sx={{ color: 'nym.text.muted', maxWidth: '220px' }}>
                Change your existing password. A strong password should have at least 8 characters, one capital letter,
                a number and a special character
              </Typography>
            ) : (
              <Typography variant="caption" sx={{ color: 'nym.text.muted', maxWidth: '220px' }}>
                Create a strong password for your wallet. A strong password should have at least 8 characters, one
                capital letter, a number and a special character
              </Typography>
            )}
          </Stack>
        </Grid>
        <Grid item sm={12} md={6} lg={4}>
          {!passwordExists && <PasswordCreateForm onPwdSaved={() => onPasswordCreated()} />}
          {passwordExists && <PasswordUpdateForm onPwdSaved={() => setModalOpen(true)} />}
        </Grid>
      </Grid>
    </>
  );
};

export default SecuritySettings;
