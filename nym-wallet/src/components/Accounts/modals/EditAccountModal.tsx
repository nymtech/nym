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
import { StyledBackButton } from 'src/components/StyledBackButton';

export const EditAccountModal = () => {
  const { accountToEdit, dialogToDisplay, setDialogToDisplay, handleEditAccount, handleAccountToEdit } =
    useContext(AccountsContext);
  const [accountName, setAccountName] = useState('');

  const theme = useTheme();
  useEffect(() => {
    if (accountToEdit) {
      setAccountName(accountToEdit.id);
    }
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
            <Typography variant="h6">Rename account</Typography>
            <IconButton onClick={() => setDialogToDisplay('Accounts')}>
              <Close />
            </IconButton>
          </Box>
        </DialogTitle>
        <DialogContent sx={{ p: 0 }}>
          <Box sx={{ px: 3, mt: 1 }}>
            <Typography sx={{ mb: 2 }}>Type the new name for your account</Typography>
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
        <DialogActions sx={{ p: 3, gap: 2 }}>
          <StyledBackButton
            onBack={() => {
              handleAccountToEdit(undefined);
              setDialogToDisplay('Accounts');
            }}
          />
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
            Rename
          </Button>
        </DialogActions>
      </Paper>
    </Dialog>
  );
};
