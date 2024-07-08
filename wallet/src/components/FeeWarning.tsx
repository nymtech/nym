import React, { useContext } from 'react';
import { Warning } from '@mui/icons-material';
import { FeeDetails } from '@nymproject/types';
import { Alert, AlertTitle, Box } from '@mui/material';
import { isBalanceEnough } from '@src/utils';
import { AppContext } from '@src/context';

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

export const BalanceWarning = ({ tx, fee }: { fee: string; tx?: string }) => {
  const { userBalance } = useContext(AppContext);

  const hasEnoughBalanace = isBalanceEnough(fee, tx, userBalance.balance?.amount.amount);

  if (hasEnoughBalanace) return null;

  return (
    <Alert color="warning" icon={<Warning />}>
      <AlertTitle>Warning: Transaction amount is greater than your balance</AlertTitle>
      The transaction amount (inc fees) is greater than your current balance, which could cause this transaction to
      fail.
      <Box sx={{ mt: 0.5 }}>Do you want to continue?</Box>
    </Alert>
  );
};
