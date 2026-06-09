import * as React from 'react';
import { useEffect } from 'react';
import { Alert, Typography } from '@mui/material';
import { TBondedNode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { isGateway, isMixnode } from 'src/types';
import { formatCoinDisplay, formatOperatorUnbondReturn } from 'src/utils/formatOperatorUnbondReturn';
import { ModalFee } from '../../Modals/ModalFee';
import { ModalListItem } from '../../Modals/ModalListItem';
import { SimpleModal } from '../../Modals/SimpleModal';
import {
  simulateUnbondGateway,
  simulateUnbondMixnode,
  simulateVestingUnbondGateway,
  simulateVestingUnbondMixnode,
} from '../../../requests';

interface Props {
  node: TBondedNode;
  onConfirm: () => Promise<void>;
  onClose: () => void;
  onError: (e: string) => void;
}

export const UnbondModal = ({ node, onConfirm, onClose, onError }: Props) => {
  const { fee, isFeeLoading, getFee, feeError } = useGetFee();
  const unbondReturn = formatOperatorUnbondReturn(node);
  const compoundedRewards = unbondReturn.hasCompoundedRewards ? unbondReturn.operatorRewards : null;

  useEffect(() => {
    if (feeError) {
      onError(feeError);
    }
  }, [feeError]);

  useEffect(() => {
    if (isMixnode(node) && !node.proxy) {
      getFee(simulateUnbondMixnode, {});
    }

    if (isMixnode(node) && node.proxy) {
      getFee(simulateVestingUnbondMixnode, {});
    }

    if (isGateway(node) && !node.proxy) {
      getFee(simulateUnbondGateway, {});
    }

    if (isGateway(node) && node.proxy) {
      getFee(simulateVestingUnbondGateway, {});
    }
  }, [node]);

  return (
    <SimpleModal
      open
      dense
      accent="primary"
      header="Unbond"
      subHeader="Unbond and remove your node from the mixnet"
      okLabel="Unbond"
      onOk={onConfirm}
      onClose={onClose}
    >
      {unbondReturn.parseError && (
        <Alert severity="warning" sx={{ mb: 1 }}>
          Could not calculate exact return - check your wallet balance after unbonding.
        </Alert>
      )}
      {compoundedRewards ? (
        <>
          <ModalListItem label="Original pledge" value={formatCoinDisplay(unbondReturn.pledge)} divider />
          <ModalListItem label="Compounded operator rewards" value={formatCoinDisplay(compoundedRewards)} divider />
          <ModalListItem label="Total returned to your account" value={formatCoinDisplay(unbondReturn.total)} divider />
          <Typography fontSize="small" sx={{ mb: 1 }}>
            Delegator stake is returned to delegators separately and is not included in this total.
          </Typography>
        </>
      ) : (
        <ModalListItem label="Total to unbond" value={formatCoinDisplay(unbondReturn.total)} divider />
      )}
      <ModalFee isLoading={isFeeLoading} fee={fee} divider />
      <Typography fontSize="small">Tokens will be transferred to the account you are logged in with now</Typography>
    </SimpleModal>
  );
};
