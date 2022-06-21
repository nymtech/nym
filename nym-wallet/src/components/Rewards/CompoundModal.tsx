import React from 'react';
import { Stack, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { SimpleModal } from '../Modals/SimpleModal';
import { FeeWarning } from '../FeeWarning';

export const CompoundModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string) => void;
  identityKey: string;
  amount: number;
  fee: number;
  currency: string;
  message: string;
}> = ({ open, onClose, onOk, identityKey, amount, fee, currency, message }) => {
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
      subHeader="Compound rewards from delegations"
      okLabel="Compound rewards"
    >
      {identityKey && <IdentityKeyFormField readOnly fullWidth initialValue={identityKey} showTickOnValid={false} />}

      <Stack direction="row" justifyContent="space-between" mb={4} mt={identityKey && 4}>
        <Typography>Rewards amount:</Typography>
        <Typography>
          {amount} {currency}
        </Typography>
      </Stack>

      <Typography mb={5} fontSize="smaller">
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
      <FeeWarning amount={amount} fee={fee} />
    </SimpleModal>
  );
};
