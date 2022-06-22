import React from 'react';
import { Button, Dialog, DialogActions, DialogContent, DialogTitle } from '@mui/material';

interface Props {
  open: boolean;
  onClose: () => void;
  children?: React.ReactNode;
  title: string;
  confirmText: string;
}

export const SimpleDialog = ({ open, onClose, children, title, confirmText }: Props) => (
  <Dialog
    open={open}
    onClose={onClose}
    aria-labelledby="responsive-dialog-title"
    maxWidth="sm"
    sx={{ textAlign: 'center' }}
  >
    <DialogTitle id="responsive-dialog-title">{title}</DialogTitle>
    <DialogContent>{children}</DialogContent>
    <DialogActions sx={{ px: 2, pb: 2 }}>
      <Button onClick={onClose} variant="contained" autoFocus fullWidth>
        {confirmText}
      </Button>
    </DialogActions>
  </Dialog>
);
