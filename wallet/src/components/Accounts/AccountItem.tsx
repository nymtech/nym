import React, { useContext } from 'react';
import EditIcon from '@mui/icons-material/Create';
import {
  Box,
  IconButton,
  ListItem,
  ListItemAvatar,
  ListItemButton,
  ListItemText,
  Tooltip,
  Typography,
} from '@mui/material';
import { useClipboard } from 'use-clipboard-copy';
import { AccountsContext } from '@src/context';
import { AccountAvatar } from './AccountAvatar';

export const AccountItem = ({
  name,
  address,
  onSelectAccount,
}: {
  name: string;
  address: string;
  onSelectAccount: () => void;
}) => {
  const { selectedAccount, setDialogToDisplay, setAccountMnemonic, handleAccountToEdit } = useContext(AccountsContext);
  const { copy, copied } = useClipboard({ copiedTimeout: 1000 });
  return (
    <ListItem
      disablePadding
      disableGutters
      sx={selectedAccount?.id === name ? { bgcolor: 'rgba(33, 208, 115, 0.1)' } : {}}
      secondaryAction={
        <IconButton
          sx={{ mr: 2, color: 'nym.text.dark' }}
          onClick={() => {
            handleAccountToEdit(name);
            setDialogToDisplay('Edit');
          }}
        >
          <EditIcon fontSize="small" />
        </IconButton>
      }
    >
      <ListItemButton disableRipple onClick={onSelectAccount}>
        <ListItemAvatar sx={{ minWidth: 0, mr: 2 }}>
          <AccountAvatar name={name} />
        </ListItemAvatar>
        <ListItemText
          primary={name}
          secondary={
            <Box>
              <Tooltip title={copied ? 'Copied!' : `Click to copy address ${address}`}>
                <Typography
                  component="span"
                  variant="body2"
                  onClick={(e: React.MouseEvent<HTMLElement>) => {
                    e.stopPropagation();
                    copy(address);
                  }}
                  sx={{ '&:hover': { color: 'grey.900' } }}
                >
                  {address}
                </Typography>
              </Tooltip>
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
      </ListItemButton>
    </ListItem>
  );
};
