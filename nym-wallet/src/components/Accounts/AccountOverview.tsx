import React from 'react';
import { Button } from '@mui/material';
import { AccountEntry } from '@nymproject/types';
import { AccountAvatar } from './AccountAvatar';

export const AccountOverview = ({ account, onClick }: { account: AccountEntry; onClick: () => void }) => (
  <Button
    startIcon={<AccountAvatar name={account.id} />}
    sx={{
      color: 'text.primary',
      '&:hover': (t) =>
        t.palette.mode === 'dark'
          ? {
              backgroundColor: 'rgba(255, 255, 255, 0.08)',
            }
          : {},
    }}
    onClick={onClick}
    disableRipple
  >
    {account.id}
  </Button>
);
