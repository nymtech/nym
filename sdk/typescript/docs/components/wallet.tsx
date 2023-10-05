import React, { useCallback, useEffect, useState, createContext } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { settings } from './client';
import { ConnectWallet } from './wallet/connect';
import { SendTokes } from './wallet/sendTokens';
import { Delegations } from './wallet/delegations';
import { WalletContextProvider } from './context/wallet';


export const Wallet = ({ type }: { type: 'connect' | 'sendTokens' | 'delegations' }) => {
  const [signerCosmosWasmClient, setSignerCosmosWasmClient] = useState<any>();
  const [signerClient, setSignerClient] = useState<any>();
  const [account, setAccount] = useState<string>();
  return (
    <WalletContextProvider>
      <Box padding={3}>
        {type === 'connect' && (
            <ConnectWallet />
        )}
        {/* {type === 'sendTokens' && (
        <WalletContext.Provider value={{...WalletContext}}>
          <SendTokes
          // setRecipientAddress={setRecipientAddress}
          // signerCosmosWasmClient={signerCosmosWasmClient}
          // account={account}
          // recipientAddress={recipientAddress}
          />
        </WalletContext.Provider>
      )}
      {type === 'delegations' && (
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
};
