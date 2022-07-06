import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { TTransactionDetails } from './types';
import { ConfirmationModal } from '../Modals/ConfirmationModal';

export const SendSuccessModal = ({ txDetails, onClose }: { txDetails: TTransactionDetails; onClose: () => void }) => (
  <ConfirmationModal open onConfirm={onClose} onClose={onClose} title="" confirmButton="Done" maxWidth="xs" fullWidth>
    <Stack alignItems="center" spacing={2}>
      <Typography>You sent</Typography>
      {txDetails && (
        <>
          <Typography variant="h5">{txDetails.amount}</Typography>
          <Link href={txDetails.txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />
        </>
      )}
    </Stack>
  </ConfirmationModal>
);
