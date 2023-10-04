import React, { useState } from 'react';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import { TableBody, TableCell, TableHead, TableRow, TextField, Typography } from '@mui/material';
import Table from '@mui/material/Table';

export const Delegations = ({
  delegations,
  doDelegate,
  delegationLoader,
  doUndelegateAll,
  undeledationLoader,
  doWithdrawRewards,
  withdrawLoading,
}: {
  delegations: any;
  doDelegate: ({ mixId, amount }: { mixId: number; amount: number }) => void;
  delegationLoader: boolean;
  doUndelegateAll: () => void;
  undeledationLoader: boolean;
  doWithdrawRewards: () => void;
  withdrawLoading: boolean;
}) => {
  const [delegationNodeId, setDelegationNodeId] = useState<string>();
  const [amountToBeDelegated, setAmountToBeDelegated] = useState<string>();

  return (
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
                onClick={() =>
                  doDelegate({ mixId: parseInt(delegationNodeId, 10), amount: parseInt(amountToBeDelegated, 10) })
                }
                disabled={delegationLoader}
              >
                {delegationLoader ? 'Delegation in process...' : 'Delegate'}
              </Button>
            </Box>
          </Box>
        </Box>
        <Box marginTop={3}>
          <Typography variant="body1">Your delegations</Typography>
          <Box marginBottom={3} display="flex" flexDirection="column">
            {!delegations?.delegations?.length ? (
              <Typography>You do not have delegations</Typography>
            ) : (
              <Box>
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
          {delegations && (
            <Box marginBottom={3}>
              <Button variant="outlined" onClick={() => doUndelegateAll()} disabled={undeledationLoader}>
                {undeledationLoader ? 'Undelegating...' : 'Undelegate All'}
              </Button>
            </Box>
          )}
          <Box>
            <Button variant="outlined" onClick={() => doWithdrawRewards()} disabled={withdrawLoading}>
              {withdrawLoading ? 'Doing withdraw...' : 'Withdraw rewards'}
            </Button>
          </Box>
        </Box>
      </Box>
    </Paper>
  );
};
