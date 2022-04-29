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
import { Close } from '@mui/icons-material';
import { AccountEntry } from 'src/types';

export const EditAccountModal = ({
  account,
  show,
  onClose,
  onEdit,
}: {
  account?: AccountEntry;
  show: boolean;
  onClose: () => void;
  onEdit: (account: AccountEntry) => void;
}) => {
  const [accountName, setAccountName] = useState('');

  useEffect(() => {
    setAccountName(account ? account?.id : '');
  }, [account]);

  return (
    <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Edit account name</Typography>
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
          onClick={() => account && onEdit({ ...account, id: accountName })}
          disabled={!accountName?.length}
        >
          Edit
        </Button>
      </DialogActions>
    </Dialog>
  );
};
