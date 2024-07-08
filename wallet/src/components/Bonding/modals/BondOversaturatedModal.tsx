import React from 'react';
import { Stack, Typography } from '@mui/material';
import { SimpleModal } from '@src/components/Modals/SimpleModal';

export const BondOversaturatedModal: FCWithChildren<{
  open: boolean;
  saturationPercentage: string;
  onClose?: () => void;
  onContinue?: () => void;
}> = ({ open, saturationPercentage, onClose, onContinue }) => (
  <SimpleModal
    open={open}
    onClose={onClose}
    onOk={async () => onContinue?.()}
    header="Change bond amount"
    okLabel="Change bond"
    buttonFullWidth
  >
    <Stack spacing={3} marginBottom={3}>
      <Typography
        fontWeight={600}
        sx={{ color: (t) => t.palette.nym.red }}
      >{`Your node saturation: ${saturationPercentage}%`}</Typography>
      <Typography fontSize={14}>Your node is over saturated, are you sure you want to bond more?</Typography>
    </Stack>
  </SimpleModal>
);
