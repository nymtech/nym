import * as React from 'react';
import { useEffect } from 'react';
import { Typography } from '@mui/material';
import { TBondedNode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { isGateway, isMixnode, isNymNode } from 'src/types';
import { ModalFee } from '../../Modals/ModalFee';
import { ModalListItem } from '../../Modals/ModalListItem';
import { SimpleModal } from '../../Modals/SimpleModal';
import {
  simulateUpdateMixnodeCostParams,
  updateMixnodeCostParams,
} from '../../../requests';
import { CurrencyDenom, DecCoin, GatewayConfigUpdate, NodeCostParams } from '@nymproject/types';

interface Props {
  node: TBondedNode;
  intervalOperatingCost: string;
  profitMarginPercent: string;
  onConfirm: () => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}

export const UpdateCostParametersModal = ({ 
  node, 
  intervalOperatingCost, 
  profitMarginPercent, 
  onConfirm, 
  onClose, 
  onError 
}: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError, onError]);

  useEffect(() => {
    try {
      const costParams: NodeCostParams = {
        profit_margin_percent: profitMarginPercent,
        interval_operating_cost: {
          denom: 'unym' as CurrencyDenom, 
          amount: intervalOperatingCost
        }
      };

      getFee(simulateUpdateMixnodeCostParams, costParams);
      
    } catch (error) {
      onError(error as string);
    }
  }, [node, intervalOperatingCost, profitMarginPercent, getFee, onError]);

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
