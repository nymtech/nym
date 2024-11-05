import React, { useContext, useEffect } from 'react';
import { Box, SxProps } from '@mui/material';
import { FeeDetails } from '@nymproject/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateUndelegateFromNode, simulateVestingUndelegateFromMixnode } from 'src/requests';
import { AppContext } from 'src/context';
import { ModalFee } from '../Modals/ModalFee';
import { ModalListItem } from '../Modals/ModalListItem';
import { SimpleModal } from '../Modals/SimpleModal';
import { BalanceWarning } from '../FeeWarning';

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
  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    if (usesVestingContractTokens) getFee(simulateVestingUndelegateFromMixnode, { mixId });
    else {
      getFee(simulateUndelegateFromNode, mixId);
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
        {userBalance.balance?.amount.amount && fee?.amount?.amount && (
          <Box sx={{ my: 2 }}>
            <BalanceWarning fee={fee?.amount?.amount} />
          </Box>
        )}
      </Box>
    </SimpleModal>
  );
};
