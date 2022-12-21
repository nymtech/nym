import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Box } from '@mui/material';
import { TestAndEarnEnterWalletAddress } from './TestAndEarnEnterWalletAddress';
import { TestAndEarnContextProvider } from './context/TestAndEarnContext';
import { NymShipyardTheme } from '../../theme';

export default {
  title: 'Growth/TestAndEarn/Components/Enter wallet address',
  component: TestAndEarnEnterWalletAddress,
} as ComponentMeta<typeof TestAndEarnEnterWalletAddress>;

export const Empty = () => (
  <NymShipyardTheme>
    <TestAndEarnContextProvider>
      <Box minWidth="25vw" maxWidth={500}>
        <TestAndEarnEnterWalletAddress sx={{ width: '100%' }} />
      </Box>
    </TestAndEarnContextProvider>
  </NymShipyardTheme>
);

export const ErrorValue = () => (
  <NymShipyardTheme>
    <TestAndEarnContextProvider>
      <Box minWidth="25vw" maxWidth={500}>
        <TestAndEarnEnterWalletAddress initialValue="this is a bad value" sx={{ width: '100%' }} />
      </Box>
    </TestAndEarnContextProvider>
  </NymShipyardTheme>
);

export const ValidValue = () => (
  <NymShipyardTheme>
    <TestAndEarnContextProvider>
      <Box minWidth="25vw" maxWidth={500}>
        <TestAndEarnEnterWalletAddress initialValue="n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" sx={{ width: '100%' }} />
      </Box>
    </TestAndEarnContextProvider>
  </NymShipyardTheme>
);
