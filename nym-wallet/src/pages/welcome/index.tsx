import React, { useContext, useState } from 'react';
import { Stack, Box, CircularProgress } from '@mui/material';
import { NymLogo } from '@nymproject/react';
import { CreatePassword, ExistingAccount, CreateMnemonic, VerifyMnemonic, WelcomeContent } from './pages';
import { TPages } from './types';
import { RenderPage } from './components';
import { ClientContext } from '../../context/main';

export const Welcome = () => {
  const [page, setPage] = useState<TPages>('welcome');
  const { isLoading } = useContext(ClientContext);

  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        overflow: 'auto',
        bgcolor: 'nym.background.dark',
      }}
    >
      <Box
        sx={{
          width: '100%',
          display: 'flex',
          justifyContent: 'center',
          margin: 'auto',
        }}
      >
        {' '}
        {isLoading ? (
          <CircularProgress size={72} />
        ) : (
          <Stack spacing={3} alignItems="center" sx={{ width: 1080 }}>
            <NymLogo width={75} />
            <RenderPage page={page}>
              <WelcomeContent
                onUseExisting={() => setPage('existing account')}
                onCreateAccount={() => setPage('create password')}
                page="welcome"
              />
              <CreatePassword
                onPrev={() => setPage('welcome')}
                onNext={() => {
                  setPage('create mnemonic');
                }}
                page="create password"
              />
              <CreateMnemonic
                onNext={() => setPage('verify mnemonic')}
                onPrev={() => setPage('create password')}
                onComplete={() => setPage('existing account')}
                page="create mnemonic"
              />
              <VerifyMnemonic
                onNext={() => setPage('existing account')}
                onPrev={() => setPage('create mnemonic')}
                page="verify mnemonic"
              />
              <ExistingAccount
                onPrev={() => setPage('welcome')}
                onCreatePassword={() => setPage('create password')}
                page="existing account"
              />
            </RenderPage>
          </Stack>
        )}
      </Box>
    </Box>
  );
};
