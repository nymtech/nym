import React, { useContext, useEffect } from 'react';
import { FeeDetails } from '@nymproject/types';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { ModalFee } from 'src/components/Modals/ModalFee';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateClaimOperatorReward, simulateVestingClaimOperatorReward } from 'src/requests';
import { AppContext } from 'src/context';
import { BalanceWarning } from 'src/components/FeeWarning';
import { Box } from '@mui/material';
import { TBondedNymNode } from 'src/requests/nymNodeDetails';

export const RedeemRewardsModal = ({
  node,
  onConfirm,
  onError,
  onClose,
}: {
  node: TBondedNymNode;
  onConfirm: (fee?: FeeDetails) => Promise<void>;
  onError: (err: string) => void;
  onClose: () => void;
}) => {
  const { fee, getFee, isFeeLoading, feeError } = useGetFee();
  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    if (feeError) onError(feeError);
  }, [feeError]);

  const handleOnOK = async () => onConfirm(fee);

  return (
    <SimpleModal
      open
      header="Claim rewards"
      subHeader="Claim you rewards"
      okLabel="Claim"
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
      <ModalListItem label="Balance" value={userBalance.balance?.printable_balance.toUpperCase()} divider />
      <ModalListItem label="Rewards will be transferred to the account you are logged in with" value="" />
      {userBalance.balance?.amount.amount && fee?.amount?.amount && (
        <Box sx={{ my: 2 }}>
          <BalanceWarning fee={fee?.amount?.amount} />
        </Box>
      )}
    </SimpleModal>
  );
};
