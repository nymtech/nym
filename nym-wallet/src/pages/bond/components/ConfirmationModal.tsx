import React from 'react';
import { Box } from '@mui/material';
import { FeeDetails, DecCoin } from '@nymproject/types';
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
  amount: DecCoin;
  fee: FeeDetails;
  onPrev: () => void;
  onConfirm: () => Promise<void>;
}) => (
  <SimpleModal header="Bond confirmation" open onOk={onConfirm} okLabel="Confirm" hideCloseIcon onBack={onPrev}>
    <Box sx={{ mt: 3 }}>
      <ModalListItem label="Mixnode identity:" value={identity} />
      <ModalListItem label="Amount:" value={`${amount.amount} ${amount.denom}`} />
      <ModalFee fee={fee} isLoading={false} />
    </Box>
  </SimpleModal>
);
