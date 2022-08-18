import React, { useContext } from 'react';
import { Box, Paper, Dialog, DialogTitle, IconButton, Typography } from '@mui/material';
import { ArrowBack } from '@mui/icons-material';
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

  return (
    <Dialog open={Boolean(accountName)} onClose={onClose} fullWidth>
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
