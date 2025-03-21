import * as React from 'react';
import { useEffect, useState } from 'react';
import { Typography } from '@mui/material';
import { TBondedNode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { ModalFee } from '../../Modals/ModalFee';
import { ModalListItem } from '../../Modals/ModalListItem';
import { SimpleModal } from '../../Modals/SimpleModal';
import {
  simulateUpdateMixnodeCostParams,
} from '../../../requests';
import { CurrencyDenom, FeeDetails, NodeCostParams } from '@nymproject/types';

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
  onFeeUpdate
}: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();
  const [hasFetchedFee, setHasFetchedFee] = useState(false);

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
        
        const uNymAmount = String(Math.floor(Number(intervalOperatingCost) * 1000000));

        const costParams: NodeCostParams = {
          profit_margin_percent: decimalProfitMargin,
          interval_operating_cost: {
            denom: 'unym' as CurrencyDenom, 
            amount: uNymAmount
          }
        };
        
        getFee(simulateUpdateMixnodeCostParams, costParams);
        
        setHasFetchedFee(true);
      } catch (error) {
        onError(error as string);
      }
    }
  }, [hasFetchedFee, intervalOperatingCost, profitMarginPercent, getFee, onError, node]);

  return (
    <SimpleModal
      open
      header="Update Cost Parameters"
      subHeader="Modify your node's economic parameters"
      okLabel="Update"
      onOk={onConfirm}
      onClose={onClose}
    >
      <ModalListItem 
        label="Interval Operating Cost" 
        value={`${intervalOperatingCost} nym`} 
        divider 
      />
      <ModalListItem 
        label="Profit Margin" 
        value={`${profitMarginPercent}%`} 
        divider 
      />
      <ModalFee isLoading={isFeeLoading} fee={fee} divider />
      <Typography fontSize="small">
        These changes will affect your node's economics and delegator rewards.
        Your new profit margin and operating cost will be applied in the next interval.
      </Typography>
    </SimpleModal>
  );
};