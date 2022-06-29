import React from 'react';
import { Box, Button, CircularProgress } from '@mui/material';
import { FeeDetails, MajorCurrencyAmount } from '@nymproject/types';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { ModalFee } from 'src/components/Modals/ModalFee';
import { ModalListItem } from 'src/components/Modals/ModalListItem';

export const ConfirmationModal = ({
  identity,
  amount,
  fee,
  onPrev,
  onConfirm,
}: {
  identity: string;
  amount: MajorCurrencyAmount;
  fee: FeeDetails;
  onPrev: () => void;
  onConfirm: () => Promise<void>;
}) => (
  <SimpleModal
    header="Bond confirmation"
    open={true}
    onOk={onConfirm}
    okLabel="Confirm"
    hideCloseIcon
    SecondaryAction={
      <Button size="large" fullWidth onClick={onPrev} sx={{ mt: 1 }}>
        Back
      </Button>
    }
  >
    <Box sx={{ mt: 3 }}>
      <>
        <ModalListItem label="Mixnode identity" value={identity} />
        <ModalListItem label="Amount" value={`${amount.amount} ${amount.denom}`} />
        <ModalFee fee={fee} isLoading={false} />
      </>
    </Box>
  </SimpleModal>
);
