import React, { useEffect, useState } from 'react';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import Alert from '@mui/material/Alert';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import { useWalletContext } from './utils/wallet.context';

export const Delegations = () => {
  const { delegations, doDelegate, delegationLoader, unDelegateAll, unDelegateAllLoading, log } = useWalletContext();

  const [delegationNodeId, setDelegationNodeId] = useState<string>();
  const [amountToBeDelegated, setAmountToBeDelegated] = useState<string>();
  const [infoText, setInfoText] = useState<string>('');

  const cleanFields = () => {
    setDelegationNodeId('');
    setAmountToBeDelegated('');
    setInfoText('');
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
          <Typography variant="h6">Delegations</Typography>
          <Box marginY={3}>
            <Box marginY={3} display="flex" flexDirection="column">
              <Typography marginBottom={3} variant="body1">
                Make a delegation
              </Typography>
              <TextField
                type="text"
                placeholder="Mixnode ID"
                onChange={(e) => setDelegationNodeId(e.target.value)}
                size="small"
              />
              <Box marginTop={3} display="flex" justifyContent="space-between">
                <TextField
                  type="text"
                  placeholder="Amount"
                  onChange={(e) => setAmountToBeDelegated(e.target.value)}
                  size="small"
                />
                <Button
                  variant="outlined"
                  onClick={() => {
                    doDelegate(delegationNodeId, amountToBeDelegated);
                    setInfoText('Changes will be visible after the next epoch');
                    cleanFields();
                  }}
                  disabled={delegationLoader}
                >
                  {delegationLoader ? 'Delegation in process...' : 'Delegate'}
                </Button>
              </Box>
            </Box>
          </Box>
          <Box marginTop={3}>
            <Typography variant="body1">Your delegations:</Typography>
            <Box marginBottom={3} display="flex" flexDirection="column">
              {!delegations?.delegations?.length ? (
                <Typography variant="body2">You do not have delegations</Typography>
              ) : (
                <Box overflow="auto">
                  <Table size="small">
                    <TableHead>
                      <TableRow>
                        <TableCell>MixId</TableCell>
                        <TableCell>Owner</TableCell>
                        <TableCell>Amount</TableCell>
                        <TableCell>Cumulative Reward Ratio</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {delegations?.delegations.map((delegation: any) => (
                        <TableRow key={delegation.mix_id}>
                          <TableCell>{delegation.mix_id}</TableCell>
                          <TableCell>{delegation.owner}</TableCell>
                          <TableCell>{delegation.amount.amount}</TableCell>
                          <TableCell>{delegation.cumulative_reward_ratio}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </Box>
              )}
            </Box>
            {delegations?.delegations.length > 0 && (
              <Box marginBottom={3}>
                <Button
                  variant="outlined"
                  onClick={() => {
                    unDelegateAll();
                    setInfoText('Changes will be visible after the next epoch');
                  }}
                  disabled={unDelegateAllLoading}
                >
                  Undelegate All
                </Button>
              </Box>
            )}

            {infoText && <Alert severity="info">{infoText}</Alert>}
          </Box>
        </Box>
      </Paper>
      {log?.node?.length > 0 && log.type === 'delegate' && (
        <Box marginTop={3}>
          <Typography variant="h5">Transaction Logs:</Typography>
          {log.node}
        </Box>
      )}
    </Box>
  );
};
