import React, { useContext, useEffect, useState } from 'react';
import { Stack, Box, CircularProgress } from '@mui/material';
import { NymLogo } from '@nymproject/react';
import { createMnemonic } from 'src/requests';
import { CreatePassword, ExistingAccount, CreateAccount, VerifyMnemonic, WelcomeContent } from './pages';
import { TMnemonicWords, TPages } from './types';
import { RenderPage } from './components';
import { ClientContext } from 'src/context/main';

const mnemonicToArray = (mnemonic: string): TMnemonicWords =>
  mnemonic
    .split(' ')
    .reduce((a, c: string, index) => [...a, { name: c, index: index + 1, disabled: false }], [] as TMnemonicWords);

export const Welcome = () => {
  const [page, setPage] = useState<TPages>('welcome');
  const [mnemonicWords, setMnemonicWords] = useState<TMnemonicWords>([]);
  const [mnemonic, setMnemonic] = useState<string>('');
  const { isLoading } = useContext(ClientContext);

  const generateMnemonic = async () => {
    const mnemonicPhrase = await createMnemonic();
    setMnemonic(mnemonicPhrase);
    setPage('create mnemonic');
  };

  useEffect(() => {
    if (mnemonic?.length > 0) {
      const mnemonicArray = mnemonicToArray(mnemonic);
      setMnemonicWords(mnemonicArray);
    }
  }, [mnemonic]);

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
                page="create password"
                onPrev={() => setPage('welcome')}
                onNext={() => setPage('create mnemonic')}
              />
              <CreateAccount
                mnemonicWords={mnemonicWords}
                mnemonic={mnemonic}
                onNext={() => setPage('verify mnemonic')}
                onPrev={() => setPage('create password')}
                onUseNew={generateMnemonic}
                page="create mnemonic"
              />
              <VerifyMnemonic
                mnemonicWords={mnemonicWords}
                onNext={() => setPage('existing account')}
                onPrev={() => setPage('create mnemonic')}
                page="verify mnemonic"
              />
              <ExistingAccount
                onPrev={() => setPage('welcome')}
                page="existing account"
                onCreatePassword={() => setPage('create password')}
              />
            </RenderPage>
          </Stack>
        )}
      </Box>
    </Box>
  );
};
