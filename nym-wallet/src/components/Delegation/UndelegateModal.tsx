import React, { useEffect, useState } from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { SimpleModal } from '../Modals/SimpleModal';
import { simulateUndelegateFromMixnode } from 'src/requests';
import { ModalListItem } from '../Modals/ModalListItem';
import { FeeDetails } from '@nymproject/types';

export const UndelegateModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, proxy: string | null) => void;
  identityKey: string;
  amount: number;
  currency: string;
  proxy: string | null;
}> = ({ identityKey, open, onClose, onOk, amount, currency, proxy }) => {
  const [fee, setFee] = useState<FeeDetails>();
  const [error, setError] = useState<string>();

  const getFee = async () => {
    try {
      const simulatedFee = await simulateUndelegateFromMixnode(identityKey);
      setFee(simulatedFee);
    } catch (e) {
      setError('Unable to determine fee estimate');
    }
  };

  useEffect(() => {
    getFee();
  }, []);

  const handleOk = async () => {
    if (onOk) {
      onOk(identityKey, proxy);
    }
  };

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
      header="Undelegate"
      subHeader="Undelegate from mixnode"
      okLabel="Undelegate stake"
      okDisabled={!fee && !error}
    >
      <IdentityKeyFormField
        readOnly
        fullWidth
        placeholder="Node identity key"
        initialValue={identityKey}
        showTickOnValid={false}
      />

      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Delegation amount" value={`${amount} ${currency}`} divider />
      </Box>

      <Typography mb={5} fontSize="smaller">
        Tokens will be transferred to account you are logged in with now
      </Typography>

      <ModalListItem
        label="Estimated fee for this operation"
        value={fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : 'n/a'}
      />
    </SimpleModal>
  );
};
