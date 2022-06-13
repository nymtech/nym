import React, { useEffect, useState } from 'react';
import { Alert, AlertTitle, Stack, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import WarningIcon from '@mui/icons-material/Warning';
import { FeeDetails } from '@nymproject/types';
import { SimpleModal } from '../Modals/SimpleModal';
import { Console } from 'src/utils/console';
import { simulateRedeemDelgatorReward } from 'src/requests';
import { ModalListItem } from '../Modals/ModalListItem';

export const RedeemModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string) => void;
  identityKey: string;
  amount: number;
  fee: number;
  minimum?: number;
  currency: string;
  message: string;
}> = ({ open, onClose, onOk, identityKey, amount, currency, message }) => {
  const [fee, setFee] = useState<FeeDetails>();

  const handleOk = async () => {
    if (onOk) {
      onOk(identityKey);
    }
  };

  const getFee = async () => {
    try {
      const simulatedfee = await simulateRedeemDelgatorReward(identityKey);
      setFee(simulatedfee);
    } catch (e) {
      Console.error(`Unable to get fee estimate for compounding reward: ${e}`);
    }
  };

  useEffect(() => {
    getFee();
  }, []);

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
      header={message}
      subHeader="Rewards from delegations"
      okLabel="Redeem rewards"
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

      <ModalListItem
        label="Estimated fee for this operation"
        value={fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : 'n/a'}
      />

      {fee?.amount && amount < +fee.amount?.amount && (
        <Alert color="warning" sx={{ mt: 3 }} icon={<WarningIcon />}>
          <AlertTitle>Warning: fees are greater than the reward</AlertTitle>
          The fees for redeeming rewards will cost more than the rewards. Are you sure you want to continue?
        </Alert>
      )}
    </SimpleModal>
  );
};
