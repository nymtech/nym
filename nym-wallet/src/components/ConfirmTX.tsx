import React from 'react';
import { SimpleModal } from './Modals/SimpleModal';
import { MajorCurrencyAmount, MajorAmountString } from '@nymproject/types';
import { ModalListItem } from './Modals/ModalListItem';
import { Button } from '@mui/material';

export const ConfirmTx: React.FC<{
  open: boolean;
  header: string;
  subheader: string;
  fee: MajorCurrencyAmount;
  currency: MajorAmountString;
  onConfirm: () => Promise<void>;
  onClose?: () => void;
  onPrev: () => void;
}> = ({ open, fee, onConfirm, onClose, header, subheader, onPrev, children }) => (
  <SimpleModal
    open={open}
    header={header}
    subHeader={subheader}
    okLabel="Confirm"
    onOk={onConfirm}
    onClose={onClose}
    SecondaryAction={
      <Button fullWidth sx={{ mt: 1 }} size="large" onClick={onPrev}>
        Cancel
      </Button>
    }
  >
    {children}
    <ModalListItem label="Estimated fee for this operation" value={`${fee.amount} ${fee.denom}`} />
  </SimpleModal>
);
