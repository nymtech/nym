import React from 'react';
import { Box, Button } from '@mui/material';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUnbondGateway, simulateVestingUnbondGateway, unbondGateway, vestingUnbondGateway } from 'src/requests';
import { EnumNodeType } from 'src/types';
import { Console } from 'src/utils/console';
import { ConfirmationModal } from './ConfirmationModal';

export const UnbondGateway = ({
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
      if (isWithVestingTokens) await getFee(simulateVestingUnbondGateway, {});
      if (!isWithVestingTokens) await getFee(simulateUnbondGateway, {});
    } catch (e) {
      Console.error(e);
      onError();
    }
  };

  const handleConfirm = async () => {
    try {
      if (isWithVestingTokens) await vestingUnbondGateway(fee?.fee);
      if (!isWithVestingTokens) await unbondGateway(fee?.fee);
      onSuccess();
    } catch (e) {
      Console.error(e);
      onError();
    }
  };

  return (
    <Box sx={{ p: 3, display: 'flex', justifyContent: 'flex-end' }}>
      {fee && (
        <ConfirmationModal fee={fee} nodeType={EnumNodeType.gateway} onPrev={resetFeeState} onConfirm={handleConfirm} />
      )}
      <Button size="large" variant="contained" disableElevation onClick={handleGetFee}>
        Unbond gateway
      </Button>
    </Box>
  );
};
