import React from 'react';
import {
  Breakpoint,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  SxProps,
  Typography,
} from '@mui/material';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import CloseIcon from '@mui/icons-material/Close';

export interface Props {
  open: boolean;
  onConfirm: () => void;
  onClose?: () => void;
  onCancel?: () => void;
  closeButton?: boolean;
  children?: React.ReactNode;
  title: React.ReactNode | string;
  subTitle?: React.ReactNode | string;
  confirmButton: React.ReactNode | string;
  cancelButton?: React.ReactNode | boolean;
  disabled?: boolean;
  sx?: SxProps;
  fullWidth?: boolean;
  maxWidth?: Breakpoint;
}

const SimpleDialog = ({
  open,
  onConfirm,
  onClose,
  children,
  title,
  subTitle,
  confirmButton,
  closeButton,
  onCancel,
  cancelButton,
  disabled,
  sx,
  fullWidth,
  maxWidth,
}: Props) => {
  const titleComp = (
    <DialogTitle id="responsive-dialog-title" sx={{ py: 3 }} color="black">
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
  const cancelButtonComp: React.ReactNode | undefined =
    cancelButton && typeof cancelButton === 'boolean' ? (
      <Button onClick={onCancel} variant="outlined" color="primary" sx={{ px: 1 }}>
        <ChevronLeftIcon />
      </Button>
    ) : (
      cancelButton
    );
  return (
    <Dialog
      open={open}
      onClose={onClose}
      aria-labelledby="responsive-dialog-title"
      maxWidth={maxWidth || 'sm'}
      sx={sx}
      fullWidth={fullWidth}
    >
      {closeButton ? (
        <Stack direction="row" alignItems="flex-start" justifyContent="space-between">
          {titleComp}
          <IconButton onClick={onClose} sx={{ mr: 2, mt: 2.6 }}>
            <CloseIcon sx={{ color: 'black' }} />
          </IconButton>
        </Stack>
      ) : (
        titleComp
      )}
      <DialogContent sx={{ pt: closeButton ? 0 : undefined }}>{children}</DialogContent>
      <DialogActions sx={{ px: 3, pb: 3 }}>
        {cancelButton ? (
          <Stack direction="row" spacing={3} width="100%">
            {cancelButtonComp}
            {confirmButtonComp}
          </Stack>
        ) : (
          confirmButtonComp
        )}
      </DialogActions>
    </Dialog>
  );
};

export default SimpleDialog;
