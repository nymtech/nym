import React, { useEffect } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyDenom, FeeDetails } from '@nymproject/types';
import { SxProps } from '@mui/material';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateClaimDelgatorReward, simulateVestingClaimDelgatorReward } from 'src/requests';
import { ModalFee } from '../Modals/ModalFee';
import { SimpleModal } from '../Modals/SimpleModal';
import { FeeWarning } from '../FeeWarning';
import { ModalListItem } from '../Modals/ModalListItem';

export const RedeemModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, fee?: FeeDetails) => void;
  identityKey: string;
  amount: number;
  denom: CurrencyDenom;
  message: string;
  sx?: SxProps;
  backdropProps?: Object;
  usesVestingTokens: boolean;
}> = ({ open, onClose, onOk, identityKey, amount, denom, message, usesVestingTokens, sx, backdropProps }) => {
  const { fee, isFeeLoading, feeError, getFee } = useGetFee();

  const handleOk = async () => {
    if (onOk) {
      onOk(identityKey, fee);
    }
  };

  useEffect(() => {
    if (usesVestingTokens) {
      getFee(simulateVestingClaimDelgatorReward, identityKey);
    } else {
      getFee(simulateClaimDelgatorReward, identityKey);
    }
  }, []);

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
      header={message}
      subHeader="Rewards from delegations"
      okLabel="Redeem rewards"
      sx={sx}
      backdropProps={backdropProps}
    >
      {identityKey && (
        <IdentityKeyFormField readOnly fullWidth initialValue={identityKey} showTickOnValid={false} sx={{ mb: 2 }} />
      )}
      <ModalListItem label="Rewards amount" value={` ${amount} ${denom.toUpperCase()}`} divider />
      {fee && <FeeWarning amount={amount} fee={fee} />}
      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} divider />
      <ModalListItem label="Rewards will be transferred to account you are logged in with now" value="" divider />
    </SimpleModal>
  );
};
