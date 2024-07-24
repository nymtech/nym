import React from 'react';
import { CoinMark, CoinMarkTestnet, NymLogo, NymLogoBW, NymWordmark } from '../dist/index';
import { NymThemeProvider } from '@nymproject/mui-theme';
import { Stack } from '@mui/system';

function App() {
  return (
    <NymThemeProvider mode="light">
      <Stack spacing={2} direction="row" justifyContent="center">
        <CoinMarkTestnet height={199} width={199} />
        <NymLogoBW height={199} width={199} />
        <NymLogo height={199} width={199} />
        <NymWordmark height={199} width={199} />
        <CoinMark height={199} width={199} />
      </Stack>
    </NymThemeProvider>
  );
}

export default App;
