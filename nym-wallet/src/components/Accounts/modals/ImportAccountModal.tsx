import React, { useContext, useState } from 'react';
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
import { AccountsContext } from 'src/context';

export const ImportAccountModal = () => {
  const [mnemonic, setMnemonic] = useState('');

  const { dialogToDisplay, setDialogToDisplay, handleImportAccount } = useContext(AccountsContext);

  const handleClose = () => {
    setMnemonic('');
    setDialogToDisplay('Accounts');
  };

  return (
    <Dialog open={dialogToDisplay === 'Import'} onClose={handleClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Import account</Typography>
          <IconButton onClick={handleClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: (theme) => theme.palette.text.disabled }}>
          Provide mnemonic of account you want to import
        </Typography>
      </DialogTitle>
      <DialogContent sx={{ p: 0 }}>
        <Box sx={{ px: 3, mt: 1 }}>
          <TextField
            placeholder="Paste or type your mnemonic here"
            fullWidth
            value={mnemonic}
            onChange={(e) => setMnemonic(e.target.value)}
            autoFocus
            multiline
            rows={3}
          />
        </Box>
      </DialogContent>
      <DialogActions sx={{ p: 3 }}>
        <Button
          fullWidth
          disableElevation
          variant="contained"
          size="large"
          onClick={() => handleImportAccount({ id: '', address: '' })}
          disabled={!mnemonic.length}
        >
          Import account
        </Button>
      </DialogActions>
    </Dialog>
  );
};
