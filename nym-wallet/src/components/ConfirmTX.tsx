import React from 'react';
import { FeeDetails } from '@nymproject/types';
import { Box } from '@mui/material';
import { SimpleModal } from './Modals/SimpleModal';
import { ModalFee } from './Modals/ModalFee';
import { ModalDivider } from './Modals/ModalDivider';

export const ConfirmTx: React.FC<{
  open: boolean;
  header: string;
  subheader?: string;
  fee: FeeDetails;
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
    onBack={onPrev}
  >
    <Box sx={{ mt: 3 }}>
      {children}
      <ModalFee fee={fee} isLoading={false} />
      <ModalDivider />
    </Box>
  </SimpleModal>
);
