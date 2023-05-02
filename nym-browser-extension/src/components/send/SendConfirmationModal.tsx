import React from 'react';
import { Box, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { ConfirmationModal, Button } from 'src/components/ui';

export const SendConfirmationModal = ({
  amount,
  txUrl,
  onConfirm,
}: {
  amount: string;
  txUrl: string;
  onConfirm: () => void;
}) => (
  <ConfirmationModal
    open
    fullWidth
    title="You sent"
    ConfirmButton={
      <Button fullWidth variant="contained" size="large" onClick={onConfirm}>
        Done
      </Button>
    }
  >
    <Box>
      <Typography variant="h6">{amount}</Typography>
      <Link href={txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />
    </Box>
  </ConfirmationModal>
);
