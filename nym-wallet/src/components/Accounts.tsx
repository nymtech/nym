import React, { useState } from 'react';
import { Button, Dialog, DialogContent, DialogTitle } from '@mui/material';
import { Circle } from '@mui/icons-material';

export type TAccount = {
  name: string;
  address: string;
};

export const Accounts = ({ accounts }: { accounts: TAccount[] }) => {
  const [addresses, setAddresses] = useState(accounts);
  const [selectedAddress, setSelectedAddress] = useState(accounts[0]);
  const [showDialog, setShowDialog] = useState(false);

  return (
    <>
      <Button startIcon={<Circle color="info" />} color="inherit" onClick={() => setShowDialog(true)}>
        {selectedAddress.name}
      </Button>
      <AccountModal show={showDialog} onClose={() => setShowDialog(false)} />
    </>
  );
};

const AccountModal = ({ show, onClose }: { show: boolean; onClose: () => void }) => {
  return (
    <Dialog open={show} onClose={onClose}>
      <DialogTitle>Yo</DialogTitle>
      <DialogContent>Content</DialogContent>
    </Dialog>
  );
};
