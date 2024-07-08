import React from 'react';
import { Stack, Typography, SxProps } from '@mui/material';
import { Link } from '@nymproject/react';
import { ConfirmationModal } from '@src/components/Modals/ConfirmationModal';
import { ErrorModal } from '@src/components/Modals/ErrorModal';

export type ConfirmationDetailProps = {
  status: 'success' | 'error';
  title: string;
  subtitle?: string;
  txUrl?: string;
};

export const ConfirmationDetailsModal = ({
  title,
  subtitle,
  children,
  txUrl,
  status,
  onClose,
  sx,
  backdropProps,
}: ConfirmationDetailProps & {
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: object;
  children?: React.ReactNode;
}) => {
  if (status === 'error') {
    <ErrorModal open message={subtitle} onClose={onClose} />;
  }

  return (
    <ConfirmationModal
      open
      onConfirm={onClose}
      onClose={onClose}
      title=""
      confirmButton="Done"
      maxWidth="xs"
      fullWidth
      sx={sx}
      backdropProps={backdropProps}
    >
      <Stack alignItems="center" spacing={2}>
        <Typography variant="h6" fontWeight={600}>
          {title}
        </Typography>
        <Typography>{subtitle}</Typography>
        {children}
        {txUrl && <Link href={txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />}
      </Stack>
    </ConfirmationModal>
  );
};
