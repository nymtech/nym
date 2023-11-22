import React from 'react';
import {
  Breakpoint,
  Button,
  Paper,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  SxProps,
  Typography,
} from '@mui/material';

export interface ConfirmationModalProps {
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
  backdropProps?: object;
}

export const ConfirmationModal = ({
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
  backdropProps,
}: ConfirmationModalProps) => {
  const Title = (
    <DialogTitle id="responsive-dialog-title" sx={{ pb: 2 }}>
      {title}
      {subTitle &&
        (typeof subTitle === 'string' ? (
          <Typography fontWeight={400} variant="subtitle1" fontSize={12} color="grey">
            {subTitle}
          </Typography>
        ) : (
          subTitle
        ))}
    </DialogTitle>
  );
  const ConfirmButton =
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
      BackdropProps={backdropProps}
      PaperComponent={Paper}
      PaperProps={{ elevation: 0 }}
    >
      {Title}
      <DialogContent>{children}</DialogContent>
      <DialogActions sx={{ px: 3, pb: 3 }}>{ConfirmButton}</DialogActions>
    </Dialog>
  );
};
