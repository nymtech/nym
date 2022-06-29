import React from 'react';
import { Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { SimpleModal } from 'src/components/Modals/SimpleModal';

export const ConfirmationModal = ({
  txUrl,
  message,
  success,
  onClose,
}: {
  txUrl?: string;
  message?: string;
  success: boolean;
  onClose: () => void;
}) => (
  <SimpleModal
    open
    header={success ? 'Successfully Unbonded' : 'Unbonding failed'}
    displayErrorIcon={!success}
    onOk={async () => onClose()}
    okLabel="Done"
    hideCloseIcon
    sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1 }}
  >
    {message && <Typography variant="caption">Error: {message}</Typography>}
    {txUrl && <Link href={txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />}
  </SimpleModal>
);
