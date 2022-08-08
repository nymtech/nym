import React, { useEffect } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyDenom, FeeDetails } from '@nymproject/types';
import { simulateCompoundDelgatorReward, simulateVestingCompoundDelgatorReward } from 'src/requests';
import { useGetFee } from 'src/hooks/useGetFee';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalFee } from '../Modals/ModalFee';
import { FeeWarning } from '../FeeWarning';
import { ModalListItem } from '../Modals/ModalListItem';

export const CompoundModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, fee?: FeeDetails) => void;
  identityKey: string;
  amount: number;
  denom: CurrencyDenom;
  message: string;
  usesVestingTokens: boolean;
}> = ({ open, onClose, onOk, identityKey, amount, denom, message, usesVestingTokens }) => {
  const { fee, isFeeLoading, feeError, getFee } = useGetFee();

  const handleOk = async () => {
    if (onOk) {
      onOk(identityKey, fee);
    }
  };

  useEffect(() => {
    if (usesVestingTokens) getFee(simulateVestingCompoundDelgatorReward, identityKey);
    else {
      getFee(simulateCompoundDelgatorReward, identityKey);
    }
  }, []);

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
      header={message}
      subHeader="Compound rewards from delegations"
      okLabel="Compound rewards"
    >
      {identityKey && (
        <IdentityKeyFormField readOnly fullWidth initialValue={identityKey} showTickOnValid={false} sx={{ mb: 2 }} />
      )}
      <ModalListItem label="Rewards amount" value={` ${amount} ${denom.toUpperCase()}`} divider />
      {fee && <FeeWarning amount={amount} fee={fee} />}
      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} divider />
      <ModalListItem label="Rewards will be added to this delegation" value="" divider />
    </SimpleModal>
  );
};
