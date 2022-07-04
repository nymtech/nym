import React, { useEffect } from 'react';
import { Alert, AlertTitle, Stack, Typography, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import WarningIcon from '@mui/icons-material/Warning';
import { simulateClaimDelgatorReward, simulateVestingClaimDelgatorReward } from 'src/requests';
import { isGreaterThan } from 'src/utils';
import { useGetFee } from 'src/hooks/useGetFee';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalFee } from '../Modals/ModalFee';
import { FeeDetails } from '@nymproject/types';

export const RedeemModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, fee?: FeeDetails) => void;
  identityKey: string;
  amount: number;
  minimum?: number;
  currency: string;
  message: string;
  sx?: SxProps;
  BackdropProps?: Object;
  usesVestingTokens: boolean;
}> = ({ open, onClose, onOk, identityKey, amount, currency, message, usesVestingTokens, sx, BackdropProps }) => {
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
      sx={{ ...sx }}
      BackdropProps={BackdropProps}
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

      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} />
      {fee?.amount && isGreaterThan(+fee.amount.amount, amount) && (
        <Alert color="warning" sx={{ mt: 3 }} icon={<WarningIcon />}>
          <AlertTitle>Warning: fees are greater than the reward</AlertTitle>
          The fees for redeeming rewards will cost more than the rewards. Are you sure you want to continue?
        </Alert>
      )}
    </SimpleModal>
  );
};
