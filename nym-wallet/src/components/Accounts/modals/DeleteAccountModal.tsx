import React, { useContext, useEffect, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  Paper,
  Typography,
} from '@mui/material';
import { Close } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { AccountsContext } from 'src/context';
import { getWalletStoragePaths } from 'src/requests';
import { formatWalletBackupReminder } from 'src/utils/walletBackupReminder';
import {
  canProceedToAccountRemovalPassword,
  formatWalletPathsLoadBlockedMessage,
} from 'src/utils/deleteAccountModalPolicy';
import { mapAccountRemovalError } from 'src/utils/accountRemovalErrors';
import { StyledBackButton } from 'src/components/StyledBackButton';
import { ConfirmPasswordModal } from './ConfirmPasswordModal';

export const DeleteAccountModal = () => {
  const {
    accountToDelete,
    dialogToDisplay,
    setDialogToDisplay,
    handleRemoveAccount,
    handleAccountToDelete,
    setError,
    error,
  } = useContext(AccountsContext);

  const [backupConfirmed, setBackupConfirmed] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [backupReminder, setBackupReminder] = useState<string>();
  const [pathsLoadError, setPathsLoadError] = useState<string>();

  const theme = useTheme();

  useEffect(() => {
    if (dialogToDisplay !== 'Delete' || !accountToDelete) {
      return;
    }

    setBackupConfirmed(false);
    setShowConfirmPassword(false);
    setPathsLoadError(undefined);
    setBackupReminder(undefined);

    getWalletStoragePaths()
      .then((paths) => setBackupReminder(formatWalletBackupReminder(paths)))
      .catch((e) => setPathsLoadError(String(e)));
  }, [dialogToDisplay, accountToDelete]);

  const handleClose = () => {
    handleAccountToDelete(undefined);
    setDialogToDisplay('Accounts');
    setBackupConfirmed(false);
    setShowConfirmPassword(false);
    setError(undefined);
  };

  const onConfirmPassword = async (password: string) => {
    if (!accountToDelete) {
      return;
    }
    try {
      await handleRemoveAccount({ account: accountToDelete, password });
      handleClose();
    } catch (e) {
      setError(mapAccountRemovalError(e));
      setShowConfirmPassword(false);
    }
  };

  const canContinue = canProceedToAccountRemovalPassword({
    backupConfirmed,
    backupReminder,
    pathsLoadError,
  });

  if (showConfirmPassword && accountToDelete) {
    return (
      <ConfirmPasswordModal
        modalTitle="Confirm account removal"
        accountName={accountToDelete.id}
        buttonTitle="Remove account permanently"
        onClose={handleClose}
        onConfirm={onConfirmPassword}
      />
    );
  }

  return (
    <Dialog
      open={dialogToDisplay === 'Delete' && Boolean(accountToDelete)}
      onClose={handleClose}
      fullWidth
      maxWidth="sm"
      PaperProps={{
        style: { border: `1px solid ${theme.palette.nym.nymWallet.modal.border}` },
      }}
    >
      <Paper>
        <DialogTitle>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Remove account</Typography>
            <IconButton onClick={handleClose}>
              <Close />
            </IconButton>
          </Box>
        </DialogTitle>
        <DialogContent sx={{ px: 3, pb: 1 }}>
          {error ? (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          ) : null}
          <Alert severity="warning" sx={{ mb: 2 }}>
            This permanently removes &quot;{accountToDelete?.id}&quot; from your saved wallet. Your password login and
            other stored accounts are kept, but this action cannot be undone.
          </Alert>

          {pathsLoadError ? (
            <Alert severity="error" sx={{ mb: 2 }}>
              {formatWalletPathsLoadBlockedMessage(pathsLoadError)}
            </Alert>
          ) : (
            <Box
              sx={{
                mb: 2,
                p: 2,
                borderRadius: 1,
                bgcolor: theme.palette.mode === 'dark' ? 'rgba(255,255,255,0.04)' : 'grey.50',
                border: `1px solid ${theme.palette.divider}`,
              }}
            >
              <Typography variant="subtitle2" sx={{ mb: 1 }}>
                Back up before you continue
              </Typography>
              <Typography
                component="pre"
                variant="body2"
                sx={{
                  whiteSpace: 'pre-wrap',
                  fontFamily: 'monospace',
                  fontSize: '0.8rem',
                  m: 0,
                }}
              >
                {backupReminder ?? 'Loading wallet file locations...'}
              </Typography>
            </Box>
          )}

          <FormControlLabel
            control={
              <Checkbox
                checked={backupConfirmed}
                onChange={(e) => setBackupConfirmed(e.target.checked)}
                color="primary"
              />
            }
            label="I have backed up my wallet file and understand this removal is permanent"
          />
        </DialogContent>
        <DialogActions sx={{ p: 3, gap: 2 }}>
          <StyledBackButton onBack={handleClose} />
          <Button
            fullWidth
            disableElevation
            variant="contained"
            color="error"
            size="large"
            disabled={!canContinue}
            onClick={() => setShowConfirmPassword(true)}
          >
            Continue to remove
          </Button>
        </DialogActions>
      </Paper>
    </Dialog>
  );
};
