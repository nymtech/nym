import React from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { ConnectWallet } from './wallet/connect';
import { SendTokes } from './wallet/sendTokens';
import { Delegations } from './wallet/delegations';
import { WalletContextProvider } from './context/wallet';

export const Wallet = ({ type }: { type: 'connect' | 'sendTokens' | 'delegations' }) => {
  return (
    <Box padding={3}>
      {type === 'connect' && (
        <WalletContextProvider>
          <ConnectWallet />
        </WalletContextProvider>
      )}
      {type === 'sendTokens' && (
        <WalletContextProvider>
          <SendTokes />
        </WalletContextProvider>
      )}
      {type === 'delegations' && (
        <WalletContextProvider>
          <Delegations />
        </WalletContextProvider>
      )}
      {/* {log.length > 0 && (
        <Box marginTop={3}>
          <Typography variant="h5">Transaction Logs:</Typography>
          {log}
        </Box>
      )} */}
    </Box>
  );
};
