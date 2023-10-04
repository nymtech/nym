import React, { useState } from 'react';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';

export const SendTokes = ({
  setRecipientAddress,
  doSendTokens,
  sendingTokensLoader,
}: {
  setRecipientAddress: (value: string) => void;
  doSendTokens: (amount: string) => void;
  sendingTokensLoader: boolean;
}) => {
  const [tokensToSend, setTokensToSend] = useState<string>();

  return (
    <Paper style={{ marginTop: '1rem', padding: '1rem' }}>
      <Box padding={3}>
        <Typography variant="h6">Send Tokens</Typography>
        <Box marginTop={3} display="flex" flexDirection="column">
          <TextField
            type="text"
            placeholder="Recipient Address"
            onChange={(e) => setRecipientAddress(e.target.value)}
            size="small"
          />
          <Box marginY={3} display="flex" justifyContent="space-between">
            <TextField
              type="text"
              placeholder="Amount"
              onChange={(e) => setTokensToSend(e.target.value)}
              size="small"
            />
            <Button variant="outlined" onClick={() => doSendTokens(tokensToSend)} disabled={sendingTokensLoader}>
              {sendingTokensLoader ? 'Sending...' : 'SendTokens'}
            </Button>
          </Box>
        </Box>
      </Box>
    </Paper>
  );
};