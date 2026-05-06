import React from 'react';
import { Button } from '@mui/material';
import { AccountEntry } from '@nymproject/types';
import { AccountAvatar } from './AccountAvatar';
import { headerControlPillSx } from '../headerControlPillSx';

export const AccountOverview = ({ account, onClick }: { account: AccountEntry; onClick: () => void }) => (
  <Button
    startIcon={<AccountAvatar name={account.id} small />}
    sx={headerControlPillSx}
    color="inherit"
    onClick={onClick}
  >
    {account.id}
  </Button>
);
