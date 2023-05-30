import React from 'react';
import { Avatar, ListItem, ListItemAvatar, ListItemText } from '@mui/material';
import { useAppContext } from 'src/context';
import { AccountActions } from './Actions';

const AccountItem = ({ accountName }: { accountName: string }) => (
  <ListItem disableGutters secondaryAction={<AccountActions accountName={accountName} />} divider>
    <ListItemAvatar>
      <Avatar>{accountName[0]}</Avatar>
    </ListItemAvatar>
    <ListItemText primary={accountName} />
  </ListItem>
);

export const AccountList = () => {
  const { accounts } = useAppContext();
  return (
    <>
      {accounts.map((accountName) => (
        <AccountItem accountName={accountName} key={accountName} />
      ))}
    </>
  );
};
