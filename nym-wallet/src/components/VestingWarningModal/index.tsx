import React, { FC } from 'react';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogContentText from '@mui/material/DialogContentText';
import DialogTitle from '@mui/material/DialogTitle';

export const VestingWarningModal: FC<{
  kind: 'delegations' | 'bond';
  isVisible: boolean;
  handleClose: (result: 'yes' | 'no') => void;
}> = ({ kind, isVisible, handleClose }) => (
  <Dialog open={isVisible} onClose={handleClose}>
    <DialogTitle>Migrate your {kind}?</DialogTitle>
    <DialogContent>
      <DialogContentText>
        By clicking <strong>yes</strong> we will migrate your {kind} to the mixnet contract.
      </DialogContentText>
      <DialogContentText sx={{ mt: 2 }}>
        The operation will be instant, you will keep your rewards and they will continue to accumulate. Once migrated,
        you will be able to withdraw your rewards.
      </DialogContentText>
    </DialogContent>
    <DialogActions>
      <Button onClick={() => handleClose('yes')}>Yes</Button>
      <Button onClick={() => handleClose('no')} autoFocus>
        No
      </Button>
    </DialogActions>
  </Dialog>
);
