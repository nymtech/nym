import React from 'react';
import { Alert, AlertTitle, Stack, Typography, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import WarningIcon from '@mui/icons-material/Warning';
import { SimpleModal } from '../Modals/SimpleModal';

export const RedeemModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string) => void;
  identityKey: string;
  amount: number;
  fee: number;
  currency: string;
  message: string;
  sx?: SxProps;
  BackdropProps?: Object;
}> = ({ open, onClose, onOk, identityKey, amount, fee, currency, message, sx, BackdropProps }) => {
  const handleOk = () => {
    if (onOk) {
      onOk(identityKey);
    }
  };
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
        <Typography sx={{ color: (theme) => theme.palette.text.primary }}>Rewards amount:</Typography>
        <Typography sx={{ color: (theme) => theme.palette.text.primary }}>
          {amount} {currency}
        </Typography>
      </Stack>

      <Typography mb={5} fontSize="smaller" sx={{ color: (theme) => theme.palette.text.primary }}>
        Rewards will be transferred to account you are logged in with now
      </Typography>

      <Stack direction="row" justifyContent="space-between">
        <Typography fontSize="smaller" color={(theme) => theme.palette.nym.fee}>
          Est. fee for this transaction:
        </Typography>
        <Typography fontSize="smaller" color={(theme) => theme.palette.nym.fee}>
          {fee} {currency}
        </Typography>
      </Stack>

      {amount < fee && (
        <Alert color="warning" sx={{ mt: 3 }} icon={<WarningIcon />}>
          <AlertTitle>Warning: fees are greater than the reward</AlertTitle>
          The fees for redeeming rewards will cost more than the rewards. Are you sure you want to continue?
        </Alert>
      )}
    </SimpleModal>
  );
};
