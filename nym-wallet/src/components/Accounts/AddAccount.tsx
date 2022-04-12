import React, { useEffect, useState } from 'react';
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  TextField,
  Typography,
} from '@mui/material';
import { Add, Close } from '@mui/icons-material';

export const AddAccountModal = ({
  show,
  onClose,
  onAdd,
}: {
  show: boolean;
  onClose: () => void;
  onAdd: (accountName: string) => void;
}) => {
  const [accountName, setAccountName] = useState('');

  useEffect(() => {
    if (!show) setAccountName('');
  }, [show]);

  return (
    <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Add new account</Typography>
          <IconButton onClick={onClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: 'grey.600' }}>
          New wallet address
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ p: 0 }}>
        <Box sx={{ px: 3, mt: 1 }}>
          <TextField
            label="Account name"
            fullWidth
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
            autoFocus
          />
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          startIcon={<Add fontSize="small" />}
          onClick={() => onAdd(accountName)}
          disabled={!accountName.length}
        >
          Add
        </Button>
      </DialogActions>
    </Dialog>
  );
};
