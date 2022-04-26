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

export const ImportAccountModal = ({
  show,
  onClose,
  onImport,
}: {
  show: boolean;
  onClose: () => void;
  onImport: (mnemonic: string) => void;
}) => {
  const [mnemonic, setMnemonic] = useState('');

  useEffect(() => {
    if (!show) setMnemonic('');
  }, [show]);

  return (
    <Dialog open={show} onClose={onClose} fullWidth hideBackdrop>
      <DialogTitle>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">Import account</Typography>
          <IconButton onClick={onClose}>
            <Close />
          </IconButton>
        </Box>
        <Typography variant="body1" sx={{ color: 'grey.600' }}>
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
          onClick={() => onImport(mnemonic)}
          disabled={!mnemonic.length}
        >
          Import account
        </Button>
      </DialogActions>
    </Dialog>
  );
};
