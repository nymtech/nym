import React from 'react';
import { Warning } from '@mui/icons-material';
import { FeeDetails } from '@nymproject/types';
import { Alert, AlertTitle } from '@mui/material';

export const FeeWarning = ({ fee, amount }: { fee: FeeDetails; amount: number }) => {
  if (fee.amount && +fee.amount.amount > amount) {
    return (
      <Alert color="warning" sx={{ mt: 3 }} icon={<Warning />}>
        <AlertTitle>Warning: fees are greater than the reward</AlertTitle>
        The fees for redeeming rewards will cost more than the rewards. Are you sure you want to continue?
      </Alert>
    );
  }
  return null;
};
