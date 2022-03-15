import React, { useContext, useState } from 'react';
import { CircularProgress, Stack, Box } from '@mui/material';
import { NymLogo } from '@nymproject/react';
import { createMnemonic } from 'src/requests';
import { ClientContext } from 'src/context/main';
import { CreatePassword, ExistingAccount, MnemonicWords, VerifyMnemonic, WelcomeContent } from './pages';
import { TMnemonicWords, TPages } from './types';
import { RenderPage } from './components';

const mnemonicToArray = (mnemonic: string): TMnemonicWords =>
  mnemonic
    .split(' ')
    .reduce((a, c: string, index) => [...a, { name: c, index: index + 1, disabled: false }], [] as TMnemonicWords);

export const Welcome = () => {
  const [page, setPage] = useState<TPages>('welcome');
  const [mnemonicWords, setMnemonicWords] = useState<TMnemonicWords>();
  const [mnemonic, setMnemonic] = useState<string>();

  console.log(mnemonic);

  const { isLoading, setIsLoading } = useContext(ClientContext);

  const generateMnemonic = async () => {
    setIsLoading(true);
    const mnemonicPhrase = await createMnemonic();
    const mnemonicArray = mnemonicToArray(mnemonicPhrase);
    setMnemonic(mnemonicPhrase);
    setMnemonicWords(mnemonicArray);
    setIsLoading(false);
    setPage('create account');
  };

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
        {isLoading ? (
          <CircularProgress size={72} />
        ) : (
          <Stack spacing={3} alignItems="center" sx={{ width: 1080 }}>
            <NymLogo width={75} />
            <RenderPage page={page}>
              <WelcomeContent
                onUseExisting={() => setPage('existing account')}
                onNext={() => setPage('create password')}
                page="welcome"
              />
              <CreatePassword page="create password" onPrev={() => setPage('welcome')} onNext={generateMnemonic} />
              <MnemonicWords
                mnemonicWords={mnemonicWords}
                mnemonic={mnemonic}
                onNext={() => setPage('verify mnemonic')}
                onPrev={() => setPage('create password')}
                page="create account"
              />
              <VerifyMnemonic
                mnemonicWords={mnemonicWords}
                onNext={() => setPage('existing account')}
                onPrev={() => setPage('create account')}
                page="verify mnemonic"
              />
              <ExistingAccount onPrev={() => setPage('welcome')} page="existing account" />
            </RenderPage>
          </Stack>
        )}
      </Box>
    </Box>
  );
};
