import { useContext, useEffect } from 'react';
import { IdentityKeyFormField } from '@nymproject/react';
import { CurrencyDenom, FeeDetails } from '@nymproject/types';
import { Box, SxProps } from '@mui/material';
import { useGetFee } from '@src/hooks/useGetFee';
import { simulateClaimDelegatorReward, simulateVestingClaimDelegatorReward } from '@src/requests';
import { AppContext } from '@src/context';
import { ModalFee } from '../Modals/ModalFee';
import { SimpleModal } from '../Modals/SimpleModal';
import { BalanceWarning, FeeWarning } from '../FeeWarning';
import { ModalListItem } from '../Modals/ModalListItem';

export const RedeemModal: FCWithChildren<{
  open: boolean;
  onClose?: () => void;
  onOk?: (mixId: number, identityKey: string, fee?: FeeDetails) => void;
  mixId: number;
  identityKey: string;
  amount: number;
  denom: CurrencyDenom;
  message: string;
  sx?: SxProps;
  backdropProps?: object;
  usesVestingTokens: boolean;
}> = ({ open, onClose, onOk, mixId, identityKey, amount, denom, message, usesVestingTokens, sx, backdropProps }) => {
  const { fee, isFeeLoading, feeError, getFee } = useGetFee();
  const { userBalance } = useContext(AppContext);

  const handleOk = async () => {
    if (onOk) {
      onOk(mixId, identityKey, fee);
    }
  };

  useEffect(() => {
    if (usesVestingTokens) {
      getFee(simulateVestingClaimDelegatorReward, mixId);
    } else {
      getFee(simulateClaimDelegatorReward, mixId);
    }
  }, []);

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
      header={message}
      subHeader="Rewards from delegations"
      okLabel="Claim rewards"
      sx={sx}
      backdropProps={backdropProps}
    >
      {identityKey && (
        <IdentityKeyFormField readOnly fullWidth initialValue={identityKey} showTickOnValid={false} sx={{ mb: 2 }} />
      )}
      <ModalListItem label="Rewards amount" value={` ${amount} ${denom.toUpperCase()}`} divider />
      <ModalFee fee={fee} isLoading={isFeeLoading} error={feeError} divider />
      <ModalListItem label="Rewards will be transferred to account you are logged in with now" value="" divider />
      {fee && <FeeWarning amount={amount} fee={fee} />}
      {userBalance.balance?.amount.amount && fee?.amount?.amount && (
        <Box sx={{ my: 2 }}>
          <BalanceWarning fee={fee?.amount?.amount} />
        </Box>
      )}
    </SimpleModal>
  );
};
