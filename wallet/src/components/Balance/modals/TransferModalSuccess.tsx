import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react';
import { ConfirmationModal } from '@src/components';

export type TTransactionDetails = { amount: string; url: string };

export const SuccessModal = ({ tx, onClose }: { tx?: TTransactionDetails; onClose: () => void }) => (
  <ConfirmationModal
    open
    title={<Typography>Transfer to balance successful</Typography>}
    onClose={onClose}
    onConfirm={onClose}
    maxWidth="xs"
    fullWidth
    confirmButton="Done"
  >
    <Stack alignItems="center" spacing={2}>
      {tx && (
        <>
          <Typography variant="h5" textTransform="uppercase">
            {tx.amount}
          </Typography>
          <Link href={tx.url} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />
        </>
      )}
    </Stack>
  </ConfirmationModal>
);
