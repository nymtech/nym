import React, { useState } from 'react';
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, Typography } from '@mui/material';
import stc from 'string-to-color';
import { Add, Circle, Close, Edit } from '@mui/icons-material';
import { NymCard } from './NymCard';

export type TAccount = {
  name: string;
  address: string;
};

const AccountColor = ({ address }: { address: string }) => <Circle sx={{ color: stc(address) }} />;

const AccountItem = ({ name, address }: { name: string; address: string }) => (
  <NymCard
    title={
      <Box>
        <Typography>{name}</Typography>
        <Typography variant="caption">{address}</Typography>
      </Box>
    }
    noPadding
    borderless
    Icon={
      <Box sx={{ mr: 1.5 }}>
        <AccountColor address={address} />
      </Box>
    }
    Action={
      <IconButton size="small">
        <Edit fontSize="small" />
      </IconButton>
    }
  />
);

const AccountModal = ({ show, addresses, onClose }: { show: boolean; addresses: TAccount[]; onClose: () => void }) => (
  <Dialog open={show} onClose={onClose} fullWidth>
    <DialogTitle>
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Typography variant="h6">Account</Typography>
        <IconButton onClick={onClose}>
          <Close />
        </IconButton>
      </Box>
      <Typography variant="caption" sx={{ color: 'grey.600' }}>
        Switch between accounts
      </Typography>
    </DialogTitle>
    <DialogContent>
      {addresses.map(({ name, address }) => (
        <AccountItem name={name} address={address} />
      ))}
    </DialogContent>
    <DialogActions>
      <Button fullWidth variant="contained" size="large" startIcon={<Add fontSize="small" />}>
        Add new account
      </Button>
    </DialogActions>
  </Dialog>
);

export const Accounts = ({ accounts }: { accounts: TAccount[] }) => {
  const [addresses, setAddresses] = useState(accounts);
  const [selectedAddress, setSelectedAddress] = useState(accounts[0]);
  const [showDialog, setShowDialog] = useState(false);

  return (
    <>
      <Button
        startIcon={<AccountColor address={selectedAddress.address} />}
        color="inherit"
        onClick={() => setShowDialog(true)}
      >
        {selectedAddress.name}
      </Button>
      <AccountModal show={showDialog} onClose={() => setShowDialog(false)} addresses={addresses} />
    </>
  );
};
