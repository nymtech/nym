import React, { useEffect } from 'react';
import { Box, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUndelegateFromMixnode } from 'src/requests';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';
import { ModalFee } from '../Modals/ModalFee';

export const UndelegateModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, usesVestingContractTokens: boolean) => void;
  identityKey: string;
  amount: number;
  currency: string;
  usesVestingContractTokens: boolean;
}> = ({ identityKey, open, onClose, onOk, amount, currency, usesVestingContractTokens }) => {
  const { fee, isFeeLoading, feeError, getFee } = useGetFee();

  useEffect(() => {
    // Need simulateVestingUndelegateFromMixnode
    getFee(simulateUndelegateFromMixnode, identityKey);
  }, []);

  const handleOk = async () => {
    if (onOk) {
      onOk(identityKey, usesVestingContractTokens);
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
      okDisabled={!fee}
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

      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} />
    </SimpleModal>
  );
};
