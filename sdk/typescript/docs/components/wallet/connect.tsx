import React  from 'react';
import { Coin } from '@cosmjs/stargate';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';

export const ConnectWallet = ({
    setMnemonic,
    connect,
    mnemonic,
    accountLoading,
    clientLoading,
    balanceLoading,
    account,
    balance,
    connectButtonText,
  }: {
    setMnemonic: (value: string) => void;
    connect: () => void;
    mnemonic: string;
    accountLoading: boolean;
    clientLoading: boolean;
    balanceLoading: boolean;
    account: string;
    balance: Coin;
    connectButtonText: string;
  }) => {
    return (
      <Paper style={{ marginTop: '1rem', padding: '1rem' }}>
        <Typography variant="h5" textAlign="center">
          Connect to your account
        </Typography>
        <Box padding={3}>
          <Typography variant="h6">Your account</Typography>
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
              onClick={() => connect()}
              disabled={!mnemonic || accountLoading || clientLoading || balanceLoading}
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
  