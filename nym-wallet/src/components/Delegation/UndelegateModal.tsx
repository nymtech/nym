import { Box, Typography, SxProps } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import React, { useEffect } from 'react';
import { FeeDetails } from '@nymproject/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUndelegateFromMixnode, simulateVestingUndelegateFromMixnode } from 'src/requests';
import { ModalFee } from '../Modals/ModalFee';
import { ModalListItem } from '../Modals/ModalListItem';
import { SimpleModal } from '../Modals/SimpleModal';

export const UndelegateModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, usesVestingContractTokens: boolean, fee?: FeeDetails) => void;
  identityKey: string;
  amount: number;
  currency: string;
  usesVestingContractTokens: boolean;
  sx?: SxProps;
  backdropProps?: object;
}> = ({ identityKey, open, onClose, onOk, amount, currency, usesVestingContractTokens, sx, backdropProps }) => {
  const { fee, isFeeLoading, feeError, getFee } = useGetFee();

  useEffect(() => {
    if (usesVestingContractTokens) getFee(simulateVestingUndelegateFromMixnode, { identity: identityKey });
    else {
      getFee(simulateUndelegateFromMixnode, identityKey);
    }
  }, []);

  const handleOk = async () => {
    if (onOk) {
      onOk(identityKey, usesVestingContractTokens, fee);
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
      sx={{ ...sx }}
      backdropProps={backdropProps}
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

      <Typography mb={5} fontSize="smaller" sx={{ color: 'text.primary' }}>
        Tokens will be transferred to account you are logged in with now
      </Typography>

      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} />
    </SimpleModal>
  );
};
