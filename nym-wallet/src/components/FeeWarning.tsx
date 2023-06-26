import React from 'react';
import { Warning } from '@mui/icons-material';
import { FeeDetails } from '@nymproject/types';
import { Alert, AlertTitle, Box } from '@mui/material';
import { isBalanceEnough } from 'src/utils';

export const FeeWarning = ({ fee, amount }: { fee: FeeDetails; amount: number }) => {
  if (fee.amount && +fee.amount.amount > amount) {
    return (
      <Alert color="warning" sx={{ mt: 3 }} icon={<Warning />}>
        <AlertTitle>Warning: Fees are greater than the reward</AlertTitle>
        The fees for redeeming rewards will cost more than the rewards. Are you sure you want to continue?
      </Alert>
    );
  }
  return null;
};

export const BalanceWarning = ({ balance, fee }: { balance: string; fee: string }) => {
  const hasEnoughBalanace = isBalanceEnough(balance, fee);

  if (!balance || !fee || hasEnoughBalanace) return null;

  return (
    <Alert color="warning" icon={<Warning />}>
      <AlertTitle>Warning: Fees are greater than your balance</AlertTitle>
      The fess are greater than your current balance which could cause this transaction to fail.
      <Box sx={{ mt: 0.5 }}>Do you want to continue?</Box>
    </Alert>
  );
};
