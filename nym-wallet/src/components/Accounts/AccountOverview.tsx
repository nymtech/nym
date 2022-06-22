import React from 'react';
import { Button } from '@mui/material';
import { AccountEntry } from '@nymproject/types';
import { AccountAvatar } from './AccountAvatar';

export const AccountOverview = ({ account, onClick }: { account: AccountEntry; onClick: () => void }) => (
  <Button
    startIcon={<AccountAvatar name={account.id} />}
    sx={{ color: (theme) => theme.palette.text.primary }}
    onClick={onClick}
    disableRipple
  >
    {account.id}
  </Button>
);
