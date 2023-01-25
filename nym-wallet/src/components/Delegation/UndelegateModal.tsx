import { Box, SxProps } from '@mui/material';
import React, { useEffect } from 'react';
import { FeeDetails } from '@nymproject/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUndelegateFromMixnode, simulateVestingUndelegateFromMixnode } from 'src/requests';
import { ModalFee } from '../Modals/ModalFee';
import { ModalListItem } from '../Modals/ModalListItem';
import { SimpleModal } from '../Modals/SimpleModal';

export const UndelegateModal: FCWithChildren<{
  open: boolean;
  onClose?: () => void;
  onOk?: (mixId: number, identityKey: string, usesVestingContractTokens: boolean, fee?: FeeDetails) => void;
  mixId: number;
  identityKey: string;
  amount: number;
  currency: string;
  usesVestingContractTokens: boolean;
  sx?: SxProps;
  backdropProps?: object;
}> = ({ mixId, identityKey, open, onClose, onOk, amount, currency, usesVestingContractTokens, sx, backdropProps }) => {
  const { fee, isFeeLoading, feeError, getFee } = useGetFee();

  useEffect(() => {
    if (usesVestingContractTokens) getFee(simulateVestingUndelegateFromMixnode, { mixId });
    else {
      getFee(simulateUndelegateFromMixnode, mixId);
    }
  }, []);

  const handleOk = async () => {
    if (onOk) {
      onOk(mixId, identityKey, usesVestingContractTokens, fee);
    }
  };

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
      header="Undelegate"
      okLabel="Undelegate stake"
      okDisabled={!fee}
      sx={sx}
      backdropProps={backdropProps}
    >
      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Node identity" value={identityKey || '-'} divider />
        <ModalListItem label="Delegation amount" value={`${amount} ${currency.toUpperCase()}`} divider />
        <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} divider />
        <ModalListItem label=" Tokens will be transferred to account you are logged in with now" value="" divider />
      </Box>
    </SimpleModal>
  );
};
