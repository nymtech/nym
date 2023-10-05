import React, { useCallback, useEffect, useState, createContext } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { ConnectWallet } from './wallet/connect';
import { SendTokes } from './wallet/sendTokens';
import { Delegations } from './wallet/delegations';
import { WalletContextProvider } from './context/wallet';

export const Wallet = ({ type }: { type: 'connect' | 'sendTokens' | 'delegations' }) => (
  <WalletContextProvider>
    <Box padding={3}>
      {type === 'connect' && <ConnectWallet />}
      {type === 'sendTokens' && <SendTokes />}
      {/* {type === 'delegations' && (
        <WalletContext.Provider value={WalletContext}>
          <Delegations />
        </WalletContext.Provider>
      )} */}
      {/* {log.length > 0 && (
        <Box marginTop={3}>
          <Typography variant="h5">Transaction Logs:</Typography>
          {log}
        </Box>
      )} */}
    </Box>
  </WalletContextProvider>
);
