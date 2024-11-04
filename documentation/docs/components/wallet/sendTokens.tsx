import React, { useState, useEffect } from 'react';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import { useWalletContext } from './utils/wallet.context';

export const SendTokes = () => {
  const { sendingTokensLoading, sendTokens, log } = useWalletContext();

  const [recipientAddress, setRecipientAddress] = useState<string>();
  const [tokensToSend, setTokensToSend] = useState<string>();

  const cleanFields = () => {
    setRecipientAddress('');
    setTokensToSend('');
  };

  useEffect(
    () => () => {
      cleanFields();
    },
    [],
  );

  return (
    <Box>
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
              <Button
                variant="outlined"
                onClick={() => {
                  sendTokens(recipientAddress, tokensToSend);
                  cleanFields();
                }}
                disabled={sendingTokensLoading}
              >
                {sendingTokensLoading ? 'Sending...' : 'Send tokens'}
              </Button>
            </Box>
          </Box>
        </Box>
      </Paper>

      {log?.node?.length > 0 && log.type === 'sendTokens' && (
        <Box marginTop={3}>
          <Typography variant="h5">Transaction Logs:</Typography>
          {log.node}
        </Box>
      )}
    </Box>
  );
};
