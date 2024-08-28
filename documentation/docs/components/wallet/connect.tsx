import React, { useState, useEffect } from 'react';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import { useWalletContext } from './utils/wallet.context';

export const ConnectWallet = () => {
  const { connect, balance, balanceLoading, accountLoading, account, clientsAreLoading } = useWalletContext();

  const [mnemonic, setMnemonic] = useState<string>();
  const [connectButtonText, setConnectButtonText] = useState<string>('Connect');

  useEffect(() => {
    if (accountLoading || clientsAreLoading || balanceLoading) {
      setConnectButtonText('Loading...');
    } else if (balance) {
      setConnectButtonText('Connected');
    }
    setConnectButtonText('Connect');
  }, [accountLoading, clientsAreLoading, balanceLoading]);

  return (
    <Paper style={{ marginTop: '1rem', padding: '1rem' }}>
      <Typography variant="h5" textAlign="center">
        Connect to your testnet account
      </Typography>
      <Box padding={3}>
        <Typography variant="h6">Your testnet account:</Typography>
        <Box marginY={3}>
          <Typography variant="body1" marginBottom={3}>
            Enter the mnemonic
          </Typography>
          <TextField
            type="text"
            placeholder="mnemonic"
            onChange={(e) => setMnemonic(e.target.value)}
            fullWidth
            multiline
            maxRows={4}
            sx={{ marginBottom: 3 }}
          />
          <Button
            variant="outlined"
            onClick={() => connect(mnemonic)}
            disabled={!mnemonic || accountLoading || clientsAreLoading || balanceLoading}
          >
            {connectButtonText}
          </Button>
        </Box>
        {account && balance ? (
          <Box>
            <Typography variant="body1">Address: {account}</Typography>
            <Typography variant="body1">
              Balance: {balance?.amount} {balance?.denom}
            </Typography>
          </Box>
        ) : (
          <Box>
            <Typography variant="body1">Please, enter your mnemonic to receive your account information</Typography>
          </Box>
        )}
      </Box>
    </Paper>
  );
};
