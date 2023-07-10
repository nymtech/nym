import React from 'react';
import { IconButton, ListItemIcon, ListItemText, Menu, MenuItem } from '@mui/material';
import { MoreVert, VisibilityOutlined } from '@mui/icons-material';
import { useAppContext } from 'src/context';

type ActionType = {
  title: string;
  Icon: React.ReactNode;
  onSelect: () => void;
};

const ActionItem = ({ action }: { action: ActionType }) => (
  <MenuItem dense onClick={action.onSelect}>
    <ListItemIcon>{action.Icon}</ListItemIcon>
    <ListItemText>{action.title}</ListItemText>
  </MenuItem>
);

export const AccountActions = ({ accountName }: { accountName: string }) => {
  const { setShowSeedForAccount } = useAppContext();

  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);

  const open = Boolean(anchorEl);

  const handleClick = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };
  const handleClose = () => {
    setAnchorEl(null);
  };

  const actions: Array<ActionType> = [
    {
      title: 'View seed phrase',
      Icon: <VisibilityOutlined />,
      onSelect: () => {
        setShowSeedForAccount(accountName);
      },
    },
  ];

  return (
    <>
      <IconButton onClick={handleClick}>
        <MoreVert />
      </IconButton>
      <Menu anchorEl={anchorEl} id="account-menu" open={open} onClose={handleClose} onClick={handleClose}>
        {actions.map((action) => (
          <ActionItem action={action} key={action.title} />
        ))}
      </Menu>
    </>
  );
};
