import React from 'react';
import { Button, Dialog, DialogActions, DialogContent, Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { TTransactionDetails } from '../types';

export const SuccessModal = ({ tx, onClose }: { tx?: TTransactionDetails; onClose: () => void }) => (
  <Dialog open onClose={onClose}>
    <DialogContent sx={{ width: 500 }}>
      <Stack alignItems="center" spacing={2}>
        <Typography>Transfer to balance successful</Typography>
        {tx && (
          <>
            <Typography variant="h5">{tx.amount}</Typography>
            <Link href={tx.url} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />
          </>
        )}
      </Stack>
    </DialogContent>
    <DialogActions>
      <Button fullWidth size="large" variant="contained" onClick={onClose}>
        Done
      </Button>
    </DialogActions>
  </Dialog>
);
