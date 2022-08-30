import React, { useEffect } from 'react';
import { FeeDetails } from '@nymproject/types';
import { ModalFee } from 'src/components/Modals/ModalFee';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { TBondedMixnode } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateCompoundOperatorReward, simulateVestingCompoundOperatorReward } from 'src/requests';

export const CompoundRewardsModal = ({
  node,
  onConfirm,
  onClose,
  onError,
}: {
  node: TBondedMixnode;
  onClose: () => void;
  onConfirm: (fee?: FeeDetails) => void;
  onError: (err: string) => void;
}) => {
  const { fee, getFee, feeError, isFeeLoading } = useGetFee();

  useEffect(() => {
    if (feeError) onError(feeError);
  }, [feeError]);

  useEffect(() => {
    if (node.proxy) getFee(simulateVestingCompoundOperatorReward, {});
    else getFee(simulateCompoundOperatorReward, {});
  }, []);

  const handleOnOK = async () => onConfirm(fee);

  return (
    <SimpleModal
      open
      header="Compound rewards"
      subHeader="Get more rewards by compounding"
      okLabel="Compound"
      okDisabled={isFeeLoading}
      onOk={handleOnOK}
      onClose={onClose}
    >
      <ModalListItem
        label="Rewards to redeem"
        value={
          node.operatorRewards ? `${node.operatorRewards.amount} ${node.operatorRewards.denom.toUpperCase()}` : '-'
        }
        divider
      />
      <ModalFee fee={fee} isLoading={isFeeLoading} divider />
      <ModalListItem label="Rewards will be transferred to the account you are logged in with" value="" />
    </SimpleModal>
  );
};
