import React, { useContext } from 'react';
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Typography } from '@mui/material';
import { Add, ArrowDownwardSharp, Close } from '@mui/icons-material';
import { AccountsContext } from 'src/context';
import { AccountItem } from './AccountItem';

export const AccountsModal = ({ onClose }: { onClose?: () => void }) => {
  const { accounts, dialogToDisplay, setDialogToDisplay } = useContext(AccountsContext);

  const handleClose = () => {
    setDialogToDisplay(undefined);
    onClose?.();
  };

  return (
    <Dialog open={dialogToDisplay === 'Accounts'} onClose={handleClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Accounts</Typography>
          <IconButton onClick={handleClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: 'grey.600' }}>
          Switch between accounts
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ padding: 0 }}>
        {accounts?.map(({ id, address }) => (
          <AccountItem name={id} address={address} key={address} />
        ))}
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button startIcon={<ArrowDownwardSharp />} onClick={() => setDialogToDisplay('Import')}>
          Import account
        </Button>
        <Button
          disableElevation
          variant="contained"
          startIcon={<Add fontSize="small" />}
          onClick={() => setDialogToDisplay('Add')}
        >
          Add new account
        </Button>
      </DialogActions>
    </Dialog>
  );
};
