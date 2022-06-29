import React from 'react';
import { Box, Button } from '@mui/material';
import { FeeDetails } from '@nymproject/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUnbondGateway, simulateVestingUnbondGateway } from 'src/requests';
import { ConfirmTx } from 'src/components/ConfirmTX';

export const UnbondGateway = ({
  isWithVestingTokens,
  onConfirm,
  onError,
}: {
  isWithVestingTokens: boolean;
  onConfirm: (isWithVestingTokens: boolean, fee: FeeDetails) => Promise<void>;
  onError: (err?: string) => void;
}) => {
  const { fee, getFee, resetFeeState } = useGetFee();

  const handleGetFee = async () => {
    try {
      if (isWithVestingTokens) await getFee(simulateVestingUnbondGateway, {});
      if (!isWithVestingTokens) await getFee(simulateUnbondGateway, {});
    } catch (e) {
      onError(e as string);
    }
  };

  return (
    <Box sx={{ p: 3, display: 'flex', justifyContent: 'flex-end' }}>
      {fee && (
        <ConfirmTx
          open
          fee={fee}
          header="Unbond gateway details"
          onPrev={resetFeeState}
          onClose={resetFeeState}
          onConfirm={async () => {
            onConfirm(isWithVestingTokens, fee);
            resetFeeState();
          }}
        />
      )}
      <Button size="large" variant="contained" disableElevation onClick={handleGetFee}>
        Unbond gateway
      </Button>
    </Box>
  );
};
