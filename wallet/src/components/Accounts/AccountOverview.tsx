import { Button } from '@mui/material';
import { AccountEntry } from '@nymproject/types';
import { AccountAvatar } from './AccountAvatar';

export const AccountOverview = ({ account, onClick }: { account: AccountEntry; onClick: () => void }) => (
  <Button
    startIcon={<AccountAvatar name={account.id} small />}
    sx={{ color: 'text.primary', fontSize: 14 }}
    color="inherit"
    onClick={onClick}
  >
    {account.id}
  </Button>
);
