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
    <Paper>
      <Dialog open={Boolean(accountName)} onClose={onClose} fullWidth hideBackdrop>
        <DialogTitle>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Switch account</Typography>
            <IconButton onClick={onClose}>
              <ArrowBack />
            </IconButton>
          </Box>
          <Typography variant="body1" sx={{ color: (theme) => theme.palette.text.disabled }}>
            Confirm password
          </Typography>
        </DialogTitle>
        <ConfirmPassword onConfirm={onConfirm} error={error} isLoading={isLoading} buttonTitle="Switch account" />
      </Dialog>
    </Paper>
  );
};
