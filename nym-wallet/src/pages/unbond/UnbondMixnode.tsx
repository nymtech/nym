import React from 'react';
import { Box, Button } from '@mui/material';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUnbondMixnode, simulateVestingUnbondMixnode, vestingUnbondMixnode, unbondMixNode } from 'src/requests';
import { EnumNodeType } from 'src/types';
import { Console } from 'src/utils/console';
import { ConfirmationModal } from './ConfirmationModal';

export const UnbondMixnode = ({
  isWithVestingTokens,
  onError,
  onSuccess,
}: {
  isWithVestingTokens: boolean;
  onError: () => void;
  onSuccess: () => void;
}) => {
  const { fee, getFee, resetFeeState } = useGetFee();

  const handleGetFee = async () => {
    try {
      if (isWithVestingTokens) await getFee(simulateVestingUnbondMixnode, {});
      if (!isWithVestingTokens) await getFee(simulateUnbondMixnode, {});
    } catch (e) {
      Console.error(e);
      onError();
    }
  };

  const handleConfirm = async () => {
    try {
      if (isWithVestingTokens) await vestingUnbondMixnode(fee?.fee);
      if (!isWithVestingTokens) await unbondMixNode(fee?.fee);
      onSuccess();
    } catch (e) {
      Console.error(e);
      onError();
    }
  };

  return (
    <Box sx={{ p: 3, display: 'flex', justifyContent: 'flex-end' }}>
      {fee && (
        <ConfirmationModal fee={fee} nodeType={EnumNodeType.mixnode} onPrev={resetFeeState} onConfirm={handleConfirm} />
      )}
      <Button size="large" variant="contained" disableElevation onClick={handleGetFee}>
        Unbond mixnode
      </Button>
    </Box>
  );
};
