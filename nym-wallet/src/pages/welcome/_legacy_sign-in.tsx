import React, { useContext, useState } from 'react';
import { Alert, Button, CircularProgress, Grid, Stack, Typography } from '@mui/material';
import { NymLogo } from '@nymproject/react';
import { ClientContext } from '../../context/main';

export const SignInContent: React.FC = () => {
  const [mnemonic] = useState<string>('');
  const [inputError, setInputError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);

  const { logIn } = useContext(ClientContext);

  const handleSignIn = async (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();

    setIsLoading(true);
    setInputError(undefined);

    try {
      await logIn({ type: 'mnemonic', value: mnemonic });
      setIsLoading(false);
    } catch (error: any) {
      setIsLoading(false);
      setInputError(error);
    }
  };

  return (
    <Stack spacing={3} alignItems="center" sx={{ width: '80%' }}>
      <NymLogo width={50} />
      <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Welcome to NYM</Typography>
      <Typography variant="caption" sx={{ color: 'grey.800', textTransform: 'uppercase', letterSpacing: 4 }}>
        Next generation of privacy
      </Typography>
      <Grid container direction="column" spacing={2}>
        <Grid item>
          <Button
            fullWidth
            variant="contained"
            color="primary"
            disabled={isLoading}
            endIcon={isLoading && <CircularProgress size={20} />}
            disableElevation
            size="large"
            onClick={handleSignIn}
            type="submit"
          >
            Create Account
          </Button>
        </Grid>
        <Grid item>
          <Button fullWidth variant="outlined" size="large">
            Use Existing Account
          </Button>
        </Grid>
        {inputError && (
          <Grid item sx={{ mt: 1 }}>
            <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light' }}>
              {inputError}
            </Alert>
          </Grid>
        )}
      </Grid>
    </Stack>
  );
};
