import React from 'react';
import { Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { ConfirmationModal } from 'src/components';
import { ErrorOutline } from '@mui/icons-material';

export const Confirmation = ({
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
  <ConfirmationModal
    open
    title={success ? 'Successfully Unbonded' : 'Unbonding failed'}
    onConfirm={onClose}
    onClose={onClose}
    confirmButton="Done"
  >
    {message && <Typography variant="caption">Error: {message}</Typography>}
    {!success && <ErrorOutline color="error" />}
    {txUrl && <Link href={txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />}
  </ConfirmationModal>
);
