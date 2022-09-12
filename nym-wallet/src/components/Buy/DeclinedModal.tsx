import React from 'react';
import { Typography } from '@mui/material';
import { ConfirmationModal } from '../Modals/ConfirmationModal';

export const DeclinedModal = ({ onOk, onClose }: { onOk: () => Promise<void>; onClose: () => void }) => (
  <ConfirmationModal
    open
    title="Buy Nym Terms and Conditions"
    confirmButton="Go back to Terms and Conditions"
    onConfirm={onOk}
    onClose={onClose}
  >
    <Typography>Can’t procced to buy tokens without acceptance</Typography>
  </ConfirmationModal>
);
