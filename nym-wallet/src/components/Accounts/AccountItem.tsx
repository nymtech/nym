import React from 'react';
import { IconButton, ListItem, ListItemAvatar, ListItemButton, ListItemIcon, ListItemText } from '@mui/material';
import { Edit } from '@mui/icons-material';
import { AccountColor } from './AccountColor';

export const AccountItem = ({
  name,
  address,
  selected,
  onSelect,
  onEdit,
}: {
  name: string;
  address: string;
  selected: boolean;
  onSelect: () => void;
  onEdit: () => void;
}) => (
  <ListItem disablePadding disableGutters sx={selected ? { bgcolor: 'rgba(33, 208, 115, 0.1)' } : {}}>
    <ListItemButton disableRipple onClick={onSelect}>
      <ListItemAvatar sx={{ minWidth: 0, mr: 2 }}>
        <AccountColor address={address} />
      </ListItemAvatar>
      <ListItemText primary={name} secondary={address} />
      <ListItemIcon>
        <IconButton
          onClick={(e) => {
            e.stopPropagation();
            onEdit();
          }}
        >
          <Edit />
        </IconButton>
      </ListItemIcon>
    </ListItemButton>
  </ListItem>
);
