import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Grid, Stack, Typography, Alert, AlertTitle } from '@mui/material';
import { AppContext } from '../../context';
import { isPasswordCreated } from '../../requests';
import { PasswordCreateForm, PasswordUpdateForm } from '../../components/Settings';
import { ConfirmationModal } from '../../components';

const SecuritySettings = () => {
  const [passwordExists, setPasswordExists] = useState(false);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [updateModalOpen, setUpdateModalOpen] = useState(false);

  const navigate = useNavigate();
  const { logOut } = useContext(AppContext);

  const checkForPassword = async () => {
    const hasPassword = await isPasswordCreated();
    setPasswordExists(hasPassword);
  };

  const onPasswordCreated = () => {
    setPasswordExists(true);
    setCreateModalOpen(true);
  };

  useEffect(() => {
    checkForPassword();
  }, []);

  return (
    <>
      <ConfirmationModal
        title={
          <Alert severity="success" sx={{ p: 2 }}>
            <AlertTitle> Password successfully created</AlertTitle>
          </Alert>
        }
        onClose={() => setCreateModalOpen(false)}
        onConfirm={async () => {
          await logOut();
          navigate('/');
        }}
        maxWidth="xs"
        confirmButton="Log out"
        open={createModalOpen}
      >
        <Stack alignItems="center" spacing={2}>
          <Typography>To use all the features of the wallet now, log in to it with your new password</Typography>
        </Stack>
      </ConfirmationModal>
      <ConfirmationModal
        title={
          <Alert severity="success" sx={{ p: 2 }}>
            <AlertTitle> Password successfully updated</AlertTitle>
          </Alert>
        }
        onClose={() => setUpdateModalOpen(false)}
        onConfirm={async () => {
          await logOut();
          navigate('/');
        }}
        maxWidth="xs"
        confirmButton="Log out"
        open={updateModalOpen}
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
          {passwordExists && <PasswordUpdateForm onPwdSaved={() => setUpdateModalOpen(true)} />}
        </Grid>
      </Grid>
    </>
  );
};

export default SecuritySettings;
