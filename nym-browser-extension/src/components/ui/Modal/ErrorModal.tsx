import React from 'react';
import {
  Breakpoint,
  Paper,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  SxProps,
  Typography,
} from '@mui/material';
import { Button } from '../Button';

export interface ErrorModalProps {
  open: boolean;
  children?: React.ReactNode;
  title: React.ReactNode | string;
  subtitle?: React.ReactNode | string;
  sx?: SxProps;
  fullWidth?: boolean;
  maxWidth?: Breakpoint;
  backdropProps?: object;
  onClose?: () => void;
}

export const ErrorModal = ({
  open,
  onClose,
  children,
  title,
  subtitle,
  sx,
  fullWidth,
  maxWidth,
  backdropProps,
}: ErrorModalProps) => {
  const Title = (
    <DialogTitle id="responsive-dialog-title" sx={{ pb: 2 }}>
      <Typography variant="body2" fontWeight={600}>
        {title}
      </Typography>
      {subtitle &&
        (typeof subtitle === 'string' ? (
          <Typography fontWeight={400} variant="subtitle1" fontSize={12} sx={{ color: 'grey.400' }}>
            {subtitle}
          </Typography>
        ) : (
          subtitle
        ))}
    </DialogTitle>
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
      <DialogActions sx={{ px: 3, pb: 3 }}>
        <Button variant="contained" size="large" fullWidth onClick={onClose}>
          Close
        </Button>
      </DialogActions>
    </Dialog>
  );
};
