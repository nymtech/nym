/* eslint-disable no-unused-vars */
import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, CardActions, CardContent, CardHeader, Stack, Typography } from '@mui/material';
import { createMnemonic } from '../../requests';
import { CopyToClipboard } from '../../components';
import { TPages } from './types';

export const CreateAccountContent: React.FC<{ page: TPages; showSignIn: () => void }> = ({ page, showSignIn }) => {
  const [mnemonic, setMnemonic] = useState<string>();
  const [error, setError] = useState<Error>();

  const handleCreateMnemonic = async () => {
    setError(undefined);
    try {
      const newMnemonic = await createMnemonic();
      setMnemonic(newMnemonic);
    } catch (e: any) {
      setError(e);
    }
  };

  useEffect(() => {
    handleCreateMnemonic();
  }, []);

  return (
    <Stack spacing={4} alignItems="center" sx={{ width: 700 }} id={page}>
      <Typography sx={{ color: 'common.white' }} variant="h4">
        Congratulations
      </Typography>
      <Typography sx={{ color: 'common.white' }} variant="h6">
        Account setup complete!
      </Typography>
      <Alert severity="info" variant="outlined" sx={{ color: 'info.light' }} data-testid="mnemonic-warning">
        <Typography>Please store your mnemonic in a safe place. You will need it to access your account!</Typography>
      </Alert>
      <Card variant="outlined" sx={{ bgcolor: 'transparent', p: 2, borderColor: 'common.white' }}>
        <CardHeader sx={{ color: 'common.white' }} title="Mnemonic" />
        <CardContent sx={{ color: 'common.white' }} data-testid="mnemonic-phrase">
          {mnemonic}
        </CardContent>
        <CardActions sx={{ justifyContent: 'flex-end' }}>
          <CopyToClipboard text={mnemonic || ''} light />
        </CardActions>
      </Card>
      {error && (
        <Alert severity="error" variant="outlined">
          {error}
        </Alert>
      )}
      <Button variant="contained" onClick={showSignIn} data-testid="sign-in-button" size="large" sx={{ width: 360 }}>
        Sign in
      </Button>
    </Stack>
  );
};
