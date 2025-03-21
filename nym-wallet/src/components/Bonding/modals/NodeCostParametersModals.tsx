import * as React from 'react';
import { useEffect, useState } from 'react';
import { Typography, Box, Alert } from '@mui/material';
import { TBondedNode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { CurrencyDenom, FeeDetails, NodeCostParams } from '@nymproject/types';
import { ModalFee } from '../../Modals/ModalFee';
import { ModalListItem } from '../../Modals/ModalListItem';
import { SimpleModal } from '../../Modals/SimpleModal';
import { simulateUpdateMixnodeCostParams } from '../../../requests';

interface Props {
  node: TBondedNode;
  intervalOperatingCost: string;
  profitMarginPercent: string;
  onConfirm: () => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
  onFeeUpdate?: (fee: FeeDetails) => void;
}

export const UpdateCostParametersModal = ({
  node,
  intervalOperatingCost,
  profitMarginPercent,
  onConfirm,
  onClose,
  onError,
  onFeeUpdate,
}: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();
  const [hasFetchedFee, setHasFetchedFee] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Handle fee errors
  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError, onError]);

  useEffect(() => {
    if (fee && onFeeUpdate) {
      onFeeUpdate(fee);
    }
  }, [fee, onFeeUpdate]);

  useEffect(() => {
    if (!hasFetchedFee) {
      try {
        const decimalProfitMargin = (parseFloat(profitMarginPercent) / 100).toString();

        const uNymAmount = String(Math.floor(Number(intervalOperatingCost || '0') * 1000000));

        const costParams: NodeCostParams = {
          profit_margin_percent: decimalProfitMargin,
          interval_operating_cost: {
            denom: 'unym' as CurrencyDenom,
            amount: uNymAmount,
          },
        };

        getFee(simulateUpdateMixnodeCostParams, costParams);
        setHasFetchedFee(true);
      } catch (error) {
        onError(error as string);
      }
    }
  }, [hasFetchedFee, intervalOperatingCost, profitMarginPercent, getFee, onError, node]);

  // Handle confirmation with loading state
  const handleConfirm = async () => {
    if (isSubmitting) return;

    try {
      setIsSubmitting(true);
      await onConfirm();
    } catch (error) {
      onError(error as string);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <SimpleModal
      open
      header="Update Cost Parameters"
      subHeader="Modify your node's economic parameters"
      okLabel={isSubmitting ? 'Updating...' : 'Update'}
      onOk={handleConfirm}
      onClose={onClose}
      okDisabled={isSubmitting || isFeeLoading}
    >
      <ModalListItem label="Interval Operating Cost" value={`${intervalOperatingCost || '0'} nym`} divider />
      <ModalListItem label="Profit Margin" value={`${profitMarginPercent}%`} divider />
      <ModalFee isLoading={isFeeLoading} fee={fee} divider />

      <Typography fontSize="small">
        These changes will affect your node's economics and delegator rewards. Your new profit margin and operating cost
        will be applied in the next interval.
      </Typography>

      {/* Warning message */}
      <Box mt={2}>
        <Alert severity="warning">
          <Typography variant="body2" fontWeight="medium">
            This action will overwrite your existing profit margin and operating cost settings. Only one cost parameter
            update is allowed per interval.
          </Typography>
        </Alert>
      </Box>
    </SimpleModal>
  );
};
