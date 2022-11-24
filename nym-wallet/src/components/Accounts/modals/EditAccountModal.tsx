import React, { useContext, useEffect, useState } from 'react';
import {
  Box,
  Button,
  Paper,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  TextField,
  Typography,
} from '@mui/material';
import { Close } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { AccountsContext } from 'src/context';

export const EditAccountModal = () => {
  const [accountName, setAccountName] = useState('');

  const { accountToEdit, dialogToDisplay, setDialogToDisplay, handleEditAccount } = useContext(AccountsContext);

  const theme = useTheme();

  useEffect(() => {
    setAccountName(accountToEdit ? accountToEdit?.id : '');
  }, [accountToEdit]);

  return (
    <Dialog
      open={dialogToDisplay === 'Edit'}
      onClose={() => setDialogToDisplay('Accounts')}
      fullWidth
      PaperProps={{
        style: { border: `1px solid ${theme.palette.nym.nymWallet.modal.border}` },
      }}
    >
      <Paper>
        <DialogTitle>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Edit account name</Typography>
            <IconButton onClick={() => setDialogToDisplay('Accounts')}>
              <Close />
            </IconButton>
          </Box>
          <Typography fontSize="small" sx={{ color: 'grey.600' }}>
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
              InputLabelProps={{ shrink: true }}
            />
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 3 }}>
          <Button
            fullWidth
            disableElevation
            variant="contained"
            size="large"
            onClick={() => {
              if (accountToEdit) {
                handleEditAccount({ ...accountToEdit, id: accountName });
                setDialogToDisplay('Accounts');
              }
            }}
            disabled={!accountName?.length}
          >
            Edit
          </Button>
        </DialogActions>
      </Paper>
    </Dialog>
  );
};
