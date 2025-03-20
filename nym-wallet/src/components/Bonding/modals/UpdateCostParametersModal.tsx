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
  simulateVestingUpdateMixnodeCostParams,
  simulateUpdateGatewayConfig
} from '../../../requests';

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
    const costParams = {
      intervalOperatingCost,
      profitMarginPercent
    };

    if (isMixnode(node)) {
      if (node.proxy) {
        getFee(simulateVestingUpdateMixnodeCostParams, costParams);
      } else {
        getFee(simulateUpdateMixnodeCostParams, costParams);
      }
    } else if (isNymNode(node) || isGateway(node)) {
      // For gateway or nym node, we use the gateway config update
      const update = {
        intervalOperatingCost,
        profitMarginPercent
      };
      
      getFee(
        simulateUpdateGatewayConfig, 
        { update }, 
        { intervalOperatingCost, profitMarginPercent },
        (args) => Promise.resolve({ fee: { amount: '0.01', denom: 'nym' } })
      );
    }
  }, [node, intervalOperatingCost, profitMarginPercent, getFee]);

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
        value={`${intervalOperatingCost} unym`} 
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
      </Typography>
    </SimpleModal>
  );
};