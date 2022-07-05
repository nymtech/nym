import React from 'react';
import {
  Breakpoint,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  SxProps,
  Typography,
} from '@mui/material';

export interface Props {
  open: boolean;
  onConfirm: () => void;
  onClose?: () => void;
  children?: React.ReactNode;
  title: React.ReactNode | string;
  subTitle?: React.ReactNode | string;
  confirmButton: React.ReactNode | string;
  disabled?: boolean;
  sx?: SxProps;
  fullWidth?: boolean;
  maxWidth?: Breakpoint;
}

const ConfirmationModal = ({
  open,
  onConfirm,
  onClose,
  children,
  title,
  subTitle,
  confirmButton,
  disabled,
  sx,
  fullWidth,
  maxWidth,
}: Props) => {
  const titleComp = (
    <DialogTitle id="responsive-dialog-title" sx={{ py: 3, pb: 2, fontWeight: 600 }} color="black">
      {title}
      {subTitle &&
        (typeof subTitle === 'string' ? (
          <Typography fontWeight={400} variant="subtitle1" fontSize={12} color={(t) => t.palette.nym.text.muted}>
            {subTitle}
          </Typography>
        ) : (
          subTitle
        ))}
    </DialogTitle>
  );
  const confirmButtonComp =
    typeof confirmButton === 'string' ? (
      <Button onClick={onConfirm} variant="contained" fullWidth disabled={disabled} sx={{ py: 1.6 }}>
        <Typography variant="button" fontSize="large">
          {confirmButton}
        </Typography>
      </Button>
    ) : (
      confirmButton
    );
  return (
    <Dialog
      open={open}
      onClose={onClose}
      aria-labelledby="responsive-dialog-title"
      maxWidth={maxWidth || 'sm'}
      sx={{ textAlign: 'center', ...sx }}
      fullWidth={fullWidth}
    >
      {titleComp}
      <DialogContent>{children}</DialogContent>
      <DialogActions sx={{ px: 3, pb: 3 }}>{confirmButtonComp}</DialogActions>
    </Dialog>
  );
};

export default ConfirmationModal;
