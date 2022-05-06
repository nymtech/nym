import React, { useContext } from 'react';
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
import { AccountsContext } from 'src/context';
import { AccountAvatar } from './AccountAvatar';

export const AccountItem = ({ name, address }: { name: string; address: string }) => {
  const { selectedAccount, handleSelectAccount, handleAccountToEdit, setDialogToDisplay, setAccountMnemonic } =
    useContext(AccountsContext);
  return (
    <ListItem
      disablePadding
      disableGutters
      sx={selectedAccount?.id === name ? { bgcolor: 'rgba(33, 208, 115, 0.1)' } : {}}
    >
      <ListItemButton disableRipple onClick={() => handleSelectAccount(name)}>
        <ListItemAvatar sx={{ minWidth: 0, mr: 2 }}>
          <AccountAvatar name={name} />
        </ListItemAvatar>
        <ListItemText
          primary={name}
          secondary={
            <Box>
              <Typography variant="body2">{address}</Typography>
              <Box sx={{ mt: 0.5 }}>
                <Typography
                  variant="body2"
                  component="span"
                  sx={{ textDecoration: 'underline', mb: 0.5, '&:hover': { color: 'primary.main' } }}
                  onClick={(e: React.MouseEvent<HTMLElement>) => {
                    e.stopPropagation();
                    setDialogToDisplay('Mnemonic');
                    setAccountMnemonic((accountMnemonic) => ({ ...accountMnemonic, accountName: name }));
                  }}
                >
                  Show mnemonic
                </Typography>
              </Box>
            </Box>
          }
        />
        <ListItemIcon>
          <IconButton
            onClick={(e) => {
              e.stopPropagation();
              handleAccountToEdit(name);
            }}
          >
            <Edit />
          </IconButton>
        </ListItemIcon>
      </ListItemButton>
    </ListItem>
  );
};
