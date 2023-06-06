import React from 'react';
import { Avatar, ListItem, ListItemAvatar, ListItemButton, ListItemText } from '@mui/material';
import { useNavigate } from 'react-router-dom';
import { useAppContext } from 'src/context';
import { AccountActions } from './Actions';

const AccountItem = ({
  accountName,
  disabled,
  onSelect,
}: {
  accountName: string;
  disabled: boolean;
  onSelect: () => void;
}) => (
  <ListItem disableGutters disablePadding secondaryAction={<AccountActions accountName={accountName} />} divider>
    <ListItemButton onClick={onSelect} disabled={disabled}>
      <ListItemAvatar>
        <Avatar>{accountName[0]}</Avatar>
      </ListItemAvatar>
      <ListItemText primary={accountName} secondary={disabled && '(Selected)'} />
    </ListItemButton>
  </ListItem>
);

export const AccountList = () => {
  const navigate = useNavigate();
  const { accounts, selectAccount, selectedAccount } = useAppContext();

  const handleSelectAccount = async (accountName: string) => {
    await selectAccount(accountName);
    navigate('/user/balance');
  };

  return (
    <>
      {accounts.map((accountName) => (
        <AccountItem
          disabled={selectedAccount === accountName}
          accountName={accountName}
          key={accountName}
          onSelect={() => handleSelectAccount(accountName)}
        />
      ))}
    </>
  );
};
