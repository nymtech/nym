import React, { useEffect } from 'react';
import { FeeDetails } from '@nymproject/types';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { ModalFee } from 'src/components/Modals/ModalFee';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateClaimOperatorReward, simulateVestingClaimOperatorReward } from 'src/requests';
import { TBondedMixnode } from 'src/context';

export const RedeemRewardsModal = ({
  node,
  onConfirm,
  onError,
  onClose,
}: {
  node: TBondedMixnode;
  onConfirm: (fee?: FeeDetails) => Promise<void>;
  onError: (err: string) => void;
  onClose: () => void;
}) => {
  const { fee, getFee, isFeeLoading, feeError } = useGetFee();

  useEffect(() => {
    if (feeError) onError(feeError);
  }, [feeError]);

  useEffect(() => {
    if (node.proxy) getFee(simulateVestingClaimOperatorReward, {});
    else getFee(simulateClaimOperatorReward, {});
  }, []);

  const handleOnOK = async () => onConfirm(fee);

  return (
    <SimpleModal
      open
      header="Redeem rewards"
      subHeader="Claim you rewards"
      okLabel="Redeem"
      okDisabled={isFeeLoading}
      onOk={handleOnOK}
      onClose={onClose}
    >
      <ModalListItem
        label="Rewards to redeem"
        value={`${node.operatorRewards.amount} ${node.operatorRewards.denom.toUpperCase()}`}
        divider
      />
      <ModalFee fee={fee} isLoading={isFeeLoading} divider />
      <ModalListItem label="Rewards will be transferred to the account you are logged in with" value="" />
    </SimpleModal>
  );
};
