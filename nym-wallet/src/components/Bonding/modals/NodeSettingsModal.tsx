import React, { useEffect, useState } from 'react';
import { Box, Button, FormHelperText, TextField, Typography } from '@mui/material';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { TBondedMixnode } from 'src/context';
import { Tabs } from 'src/components/Tabs';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { isDecimal } from 'src/utils';
import { useGetFee } from 'src/hooks/useGetFee';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { simulateUpdateMixnode, simulateVestingUpdateMixnode } from 'src/requests';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { FeeDetails } from '@nymproject/types';

export const NodeSettings = ({
  currentPm,
  isVesting,
  onConfirm,
  onClose,
  onError,
}: {
  currentPm: TBondedMixnode['profitMargin'];
  isVesting: boolean;
  onConfirm: (profitMargin: number, fee?: FeeDetails) => Promise<void>;
  onClose: () => void;
  onError: (err: string) => void;
}) => {
  const [pm, setPm] = useState(currentPm.toString());
  const [error, setError] = useState(false);

  const { fee, getFee, resetFeeState, isFeeLoading, feeError } = useGetFee();

  const handleValidate = async () => {
    let isValid = true;
    const pmAsNumber = Number(pm);

    if (!pmAsNumber) {
      isValid = false;
    }
    if (isDecimal(pmAsNumber)) {
      isValid = false;
    }
    if (pmAsNumber > 100) {
      isValid = false;
    }
    if (pmAsNumber < 0) {
      isValid = false;
    }

    if (!isValid) {
      setError(true);
      return;
    }

    if (isVesting) {
      await getFee(simulateVestingUpdateMixnode, { profitMarginPercent: pmAsNumber });
    } else {
      await getFee(simulateUpdateMixnode, { profitMarginPercent: pmAsNumber });
    }
  };

  useEffect(() => {
    setError(false);
  }, [pm]);

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  if (isFeeLoading) return <LoadingModal />;

  if (fee)
    return (
      <ConfirmTx
        open
        header="Profit margin change"
        fee={fee}
        onPrev={resetFeeState}
        onClose={onClose}
        onConfirm={() => onConfirm(Number(pm), fee)}
      >
        <ModalListItem label="Current profit margin" value={`${currentPm}%`} divider />
        <ModalListItem label="New profit margin" value={`${pm}%`} divider />
      </ConfirmTx>
    );

  return (
    <SimpleModal
      open
      hideCloseIcon
      sx={{ p: 0 }}
      header={
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, p: 3 }}>
          <NodeIcon />
          <Typography variant="h6" fontWeight={600}>
            Node Settings
          </Typography>
        </Box>
      }
      okLabel="Next"
      onClose={onClose}
    >
      <Tabs tabs={['System variables']} selectedTab={0} disableActiveTabHighlight />
      <Box sx={{ p: 3 }}>
        <Typography fontWeight={600} sx={{ mb: 1 }}>
          Set profit margin
        </Typography>
        <Box sx={{ mb: 3 }}>
          <TextField placeholder="Profit margin" value={pm} onChange={(e) => setPm(e.target.value)} fullWidth />
          {error && (
            <FormHelperText sx={{ color: 'error.main' }}>
              Profit margin should be a whole number between 0 and 100
            </FormHelperText>
          )}
          <FormHelperText>Your new profit margin will be applied in the next epoch</FormHelperText>
        </Box>
        <Box sx={{ mb: 3 }}>
          <ModalListItem label="Estimated operator reward for 10% profit margin" value="150 NYM" divider />
          <ModalListItem label="Est. fee for this operation will be caculated in the next page" value="" />
        </Box>
        <Button variant="contained" fullWidth size="large" onClick={handleValidate} disabled={error}>
          Next
        </Button>
      </Box>
    </SimpleModal>
  );
};
