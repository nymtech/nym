import React from 'react';
import {
  Box,
  IconButton,
  ListItem,
  ListItemAvatar,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Typography,
} from '@mui/material';
import { Edit } from '@mui/icons-material';
import { AccountAvatar } from './AccountAvatar';
import { ShowMnemonic } from './ShowMnemonic';

export const AccountItem = ({
  name,
  address,
  isSelected,
  onSelect,
  onEdit,
}: {
  name: string;
  address: string;
  isSelected: boolean;
  onSelect: () => void;
  onEdit: () => void;
}) => (
  <ListItem disablePadding disableGutters sx={isSelected ? { bgcolor: 'rgba(33, 208, 115, 0.1)' } : {}}>
    <ListItemButton disableRipple onClick={onSelect}>
      <ListItemAvatar sx={{ minWidth: 0, mr: 2 }}>
        <AccountAvatar name={name} address={address} />
      </ListItemAvatar>
      <ListItemText
        primary={name}
        secondary={
          <Box>
            <Typography variant="body2">{address}</Typography>
            <Box sx={{ mt: 0.5 }}>
              <ShowMnemonic accountName={name} />
            </Box>
          </Box>
        }
      />
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
