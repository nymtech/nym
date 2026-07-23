import React from 'react';
import { FeeDetails } from '@nymproject/types';
import { Box } from '@mui/material';
import { SimpleModal } from './Modals/SimpleModal';
import { ModalFee } from './Modals/ModalFee';
import { ModalDivider } from './Modals/ModalDivider';

export const ConfirmTx: FCWithChildren<{
  open: boolean;
  header: string;
  subheader?: string;
  fee: FeeDetails;
  onConfirm: () => Promise<void>;
  onClose?: () => void;
  onPrev: () => void;
  disableBackdropClose?: boolean;
  children?: React.ReactNode;
}> = ({ open, fee, onConfirm, onClose, header, subheader, onPrev, disableBackdropClose, children }) => (
  <SimpleModal
    open={open}
    header={header}
    subHeader={subheader}
    okLabel="Confirm"
    onOk={onConfirm}
    onClose={onClose}
    disableBackdropClose={disableBackdropClose}
    onBack={onPrev}
  >
    <Box sx={{ mt: 3 }}>
      {children}
      <ModalFee fee={fee} isLoading={false} />
      <ModalDivider />
    </Box>
  </SimpleModal>
);
