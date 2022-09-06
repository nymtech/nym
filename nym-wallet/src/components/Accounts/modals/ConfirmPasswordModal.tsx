import React, { useContext } from 'react';
import { Paper, Dialog, DialogTitle, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { ConfirmPassword } from 'src/components/ConfirmPassword';
import { AccountsContext } from 'src/context';

export const ConfirmPasswordModal = ({
  accountName,
  onClose,
  onConfirm,
}: {
  accountName?: string;
  onClose: () => void;
  onConfirm: (password: string) => Promise<void>;
}) => {
  const { isLoading, error } = useContext(AccountsContext);
  const theme = useTheme();

  return (
    <Dialog
      open={Boolean(accountName)}
      onClose={onClose}
      fullWidth
      PaperProps={{
        style: { border: `1px solid ${theme.palette.nym.nymWallet.modal.border}` },
      }}
    >
      <Paper>
        <DialogTitle>
          <Typography variant="h6">Switch account</Typography>
          <Typography fontSize="small" sx={{ color: 'grey.600' }}>
            Confirm password
          </Typography>
        </DialogTitle>
        <ConfirmPassword
          onConfirm={onConfirm}
          error={error}
          isLoading={isLoading}
          buttonTitle="Switch account"
          onCancel={onClose}
        />
      </Paper>
    </Dialog>
  );
};
