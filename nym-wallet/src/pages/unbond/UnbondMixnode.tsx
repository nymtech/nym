import React from 'react';
import { Box, Button } from '@mui/material';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUnbondMixnode, simulateVestingUnbondMixnode } from 'src/requests';
import { FeeDetails } from '@nymproject/types';

export const UnbondMixnode = ({
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
      if (isWithVestingTokens) await getFee(simulateVestingUnbondMixnode, {});
      if (!isWithVestingTokens) await getFee(simulateUnbondMixnode, {});
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
          header="Unbond mixnode details"
          onPrev={resetFeeState}
          onClose={resetFeeState}
          onConfirm={async () => {
            onConfirm(isWithVestingTokens, fee);
            resetFeeState();
          }}
        />
      )}
      <Button size="large" variant="contained" disableElevation onClick={handleGetFee}>
        Unbond mixnode
      </Button>
    </Box>
  );
};
