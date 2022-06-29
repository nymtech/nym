import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { SimpleModal } from '../Modals/SimpleModal';
import { TTransactionDetails } from './types';

export const SendSuccessModal = ({ txDetails, onClose }: { txDetails: TTransactionDetails; onClose: () => void }) => (
  <SimpleModal
    open
    onClose={onClose}
    onOk={async () => onClose()}
    okLabel="Close"
    header=""
    hideCloseIcon
    sx={{ width: 350 }}
  >
    <Stack alignItems="center" spacing={2}>
      <Typography>You sent</Typography>
      {txDetails && (
        <>
          <Typography variant="h5">{txDetails.amount}</Typography>
          <Link href={txDetails.txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />
        </>
      )}
    </Stack>
  </SimpleModal>
);
