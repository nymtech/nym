import React, { useContext, useState } from 'react';
import { NymLogo } from '@nymproject/react';
import { CircularProgress, Stack, Box } from '@mui/material';
import { ExistingAccount, WelcomeContent } from './pages';
import { TPages } from './types';
import { RenderPage } from './components';
import { CreateAccountContent } from './_legacy_create-account';
import { ClientContext } from '../../context/main';

// const testMnemonic =
//   'futuristic big receptive caption saw hug odd spoon internal dime bike rake helpless left distribution gusty eyes beg enormous word influence trashy pets curl';
//
// const mnemonicToArray = (mnemonic: string): TMnemonicWords =>
//   mnemonic
//     .split(' ')
//     .reduce((a, c: string, index) => [...a, { name: c, index: index + 1, disabled: false }], [] as TMnemonicWords);

export const Welcome = () => {
  const [page, setPage] = useState<TPages>('welcome');
  // const [mnemonicWords, setMnemonicWords] = useState<TMnemonicWords>();

  const { isLoading } = useContext(ClientContext);

  // useEffect(() => {
  //   const mnemonicArray = mnemonicToArray(testMnemonic)
  //   setMnemonicWords(mnemonicArray)
  // }, [])

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
                onCreateAccountComplete={() => setPage('legacy create account')}
                page="welcome"
              />

              <CreateAccountContent page="legacy create account" showSignIn={() => setPage('existing account')} />
              {/* <MnemonicWords
              mnemonicWords={mnemonicWords}
              onNext={() => setPage('verify mnemonic')}
              onPrev={() => setPage('welcome')}
              page="create account"
            />
            <VerifyMnemonic
              mnemonicWords={mnemonicWords}
              onComplete={() => setPage('create password')}
              page="verify mnemonic"
            />
            <CreatePassword page="create password" /> */}
              <ExistingAccount onPrev={() => setPage('welcome')} page="existing account" />
            </RenderPage>
          </Stack>
        )}
      </Box>
    </Box>
  );
};
