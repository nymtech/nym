import * as React from 'react';
import { useEffect, useState } from 'react';
import { Typography, Box, Alert } from '@mui/material';
import { TBondedNode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { CurrencyDenom, FeeDetails, NodeCostParams } from '@nymproject/types';
import { ErrorModal } from '../../Modals/ErrorModal';
import { LoadingModal } from '../../Modals/LoadingModal';
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
        onError(String(error));
      }
    }
  }, [hasFetchedFee, intervalOperatingCost, profitMarginPercent, getFee, onError]);

  const handleConfirm = async () => {
    if (isSubmitting || !fee) return;

    setIsSubmitting(true);
    try {
      await onConfirm();
    } finally {
      setIsSubmitting(false);
    }
  };

  if (isFeeLoading) {
    return <LoadingModal />;
  }

  if (feeError) {
    return <ErrorModal open title="An error occurred" message={feeError} onClose={onClose} />;
  }

  if (!fee) {
    return null;
  }

  return (
    <SimpleModal
      open
      header="Update Cost Parameters"
      subHeader="Modify your node's economic parameters"
      okLabel={isSubmitting ? 'Updating...' : 'Update'}
      onOk={handleConfirm}
      onClose={onClose}
      okDisabled={isSubmitting}
    >
      <ModalListItem label="Interval Operating Cost" value={`${intervalOperatingCost || '0'} nym`} divider />
      <ModalListItem label="Profit Margin" value={`${profitMarginPercent}%`} divider />
      <ModalFee isLoading={false} fee={fee} divider />

      <Typography fontSize="small">
        These changes will affect your node&apos;s economics and delegator rewards. Your new profit margin and operating
        cost will be applied in the next interval.
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
