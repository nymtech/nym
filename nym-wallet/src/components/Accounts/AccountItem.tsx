import React, { useContext } from 'react';
import EditIcon from '@mui/icons-material/Create';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import {
  Box,
  IconButton,
  ListItem,
  ListItemAvatar,
  ListItemButton,
  ListItemText,
  Tooltip,
  Typography,
  alpha,
  useTheme,
} from '@mui/material';
import { useClipboard } from 'use-clipboard-copy';
import { AccountsContext } from 'src/context';
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
  const theme = useTheme();
  
  const isSelected = selectedAccount?.id === name;
  
  return (
    <ListItem
      disablePadding
      disableGutters
      sx={{
        borderRadius: 2,
        my: 0.5,
        mx: 1,
        overflow: 'hidden',
        position: 'relative',
        bgcolor: isSelected
          ? alpha(theme.palette.nym.highlight, theme.palette.mode === 'dark' ? 0.15 : 0.08)
          : 'transparent',
        borderLeft: isSelected 
          ? `3px solid ${theme.palette.nym.highlight}` 
          : '3px solid transparent',
      }}
      secondaryAction={
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          {isSelected && (
            <CheckCircleIcon 
              sx={{ 
                color: theme.palette.nym.highlight,
                fontSize: 18,
              }}
            />
          )}
          <IconButton
            sx={{ 
              mr: 1.5, 
              color: theme.palette.mode === 'dark' ? 'nym.text.dark' : theme.palette.text.primary,
              backgroundColor: alpha(theme.palette.text.primary, 0.05),
              '&:hover': {
                backgroundColor: alpha(theme.palette.text.primary, 0.1),
              },
              width: 30,
              height: 30,
            }}
            onClick={() => {
              handleAccountToEdit(name);
              setDialogToDisplay('Edit');
            }}
          >
            <EditIcon fontSize="small" />
          </IconButton>
        </Box>
      }
    >
      <ListItemButton 
        disableRipple 
        onClick={onSelectAccount}
        sx={{
          py: 1,
          transition: 'background-color 0.2s',
          '&:hover': {
            backgroundColor: isSelected 
              ? alpha(theme.palette.nym.highlight, theme.palette.mode === 'dark' ? 0.2 : 0.12)
              : alpha(theme.palette.nym.nymWallet.hover.background, 0.5),
          },
        }}
      >
        {/* Account avatar with box wrapper to apply styling */}
        <ListItemAvatar 
          sx={{ 
            minWidth: 0, 
            mr: 2,
            '& .MuiAvatar-root': {
              border: isSelected 
                ? `2px solid ${theme.palette.nym.highlight}` 
                : `2px solid transparent`,
              transition: 'all 0.2s',
            }
          }}
        >
          <AccountAvatar name={name} />
        </ListItemAvatar>
        <ListItemText
          primary={
            <Typography 
              variant="subtitle1" 
              sx={{ 
                fontWeight: isSelected ? 600 : 400,
                color: theme.palette.text.primary,
              }}
            >
              {name}
            </Typography>
          }
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
                  sx={{ 
                    fontFamily: 'monospace',
                    cursor: 'pointer',
                    color: theme.palette.mode === 'dark'
                      ? theme.palette.nym.nymWallet.text.muted
                      : alpha(theme.palette.text.primary, 0.7),
                    '&:hover': { 
                      color: theme.palette.text.primary,
                      textDecoration: 'underline',
                    },
                  }}
                >
                  {address}
                </Typography>
              </Tooltip>
              <Box sx={{ mt: 0.5 }}>
                <Typography
                  variant="body2"
                  component="span"
                  sx={{ 
                    textDecoration: 'underline', 
                    mb: 0.5, 
                    cursor: 'pointer',
                    color: theme.palette.mode === 'dark'
                      ? alpha(theme.palette.nym.highlight, 0.9)
                      : theme.palette.nym.highlight,
                    '&:hover': { 
                      color: theme.palette.nym.highlight,
                      fontWeight: 500,
                    } 
                  }}
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