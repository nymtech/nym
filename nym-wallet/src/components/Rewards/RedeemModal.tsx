import React, { useEffect } from 'react';
import { Stack, Typography, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { FeeDetails } from '@nymproject/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateClaimDelgatorReward, simulateVestingClaimDelgatorReward } from 'src/requests';
import { ModalFee } from '../Modals/ModalFee';
import { SimpleModal } from '../Modals/SimpleModal';
import { FeeWarning } from '../FeeWarning';

export const RedeemModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, fee?: FeeDetails) => void;
  identityKey: string;
  amount: number;
  currency: string;
  message: string;
  sx?: SxProps;
  backdropProps?: Object;
  usesVestingTokens: boolean;
}> = ({ open, onClose, onOk, identityKey, amount, currency, message, usesVestingTokens, sx, backdropProps }) => {
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
      {identityKey && <IdentityKeyFormField readOnly fullWidth initialValue={identityKey} showTickOnValid={false} />}

      <Stack direction="row" justifyContent="space-between" mb={4} mt={identityKey && 4}>
        <Typography sx={{ color: 'text.primary' }}>Rewards amount:</Typography>
        <Typography sx={{ color: 'text.primary' }}>
          {amount} {currency}
        </Typography>
      </Stack>

      <Typography mb={5} fontSize="smaller" sx={{ color: 'text.primary' }}>
        Rewards will be transferred to account you are logged in with now
      </Typography>

      {fee && <FeeWarning amount={amount} fee={fee} />}
      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} />
    </SimpleModal>
  );
};
