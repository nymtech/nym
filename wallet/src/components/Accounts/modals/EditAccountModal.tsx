import { useContext, useEffect, useState } from 'react';
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
import { AccountsContext } from '@src/context';
import { StyledBackButton } from '@src/components/StyledBackButton';
import { ConfirmPasswordModal } from './ConfirmPasswordModal';

export const EditAccountModal = () => {
  const { accountToEdit, dialogToDisplay, setDialogToDisplay, handleEditAccount, handleAccountToEdit, setError } =
    useContext(AccountsContext);

  const [accountName, setAccountName] = useState('');
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  const theme = useTheme();

  useEffect(() => {
    if (accountToEdit) {
      setAccountName(accountToEdit.id);
    }
  }, [accountToEdit]);

  const handleClose = () => {
    handleAccountToEdit(undefined);
    setDialogToDisplay('Accounts');
  };

  const onConfirmPassword = async (password: string) => {
    if (accountToEdit) {
      try {
        await handleEditAccount({ account: accountToEdit, newAccountName: accountName, password });
        setShowConfirmPassword(false);
      } catch (e) {
        setError(`Error editing account: ${e}`);
      }
    }
  };

  if (showConfirmPassword) {
    return (
      <ConfirmPasswordModal
        modalTitle="Rename account"
        accountName={accountToEdit?.id}
        buttonTitle="Confirm"
        onClose={() => {
          setShowConfirmPassword(false);
          setError(undefined);
        }}
        onConfirm={onConfirmPassword}
      />
    );
  }

  return (
    <Dialog
      open={dialogToDisplay === 'Edit'}
      onClose={handleClose}
      fullWidth
      PaperProps={{
        style: { border: `1px solid ${theme.palette.nym.nymWallet.modal.border}` },
      }}
    >
      <Paper>
        <DialogTitle>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Rename account</Typography>
            <IconButton onClick={handleClose}>
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
          <StyledBackButton onBack={handleClose} />
          <Button
            fullWidth
            disableElevation
            variant="contained"
            size="large"
            onClick={() => {
              setShowConfirmPassword(true);
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
