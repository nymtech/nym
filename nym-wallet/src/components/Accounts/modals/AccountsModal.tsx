import React, { useContext, useState } from 'react';
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Typography,
  Divider,
  alpha,
  useTheme,
  List,
} from '@mui/material';
import { Add, ArrowDownwardSharp, Close, SwapHorizOutlined } from '@mui/icons-material';
import { AccountsContext } from 'src/context';
import { AccountItem } from '../AccountItem';
import { ConfirmPasswordModal } from './ConfirmPasswordModal';

export const AccountsModal = () => {
  const { accounts, dialogToDisplay, setDialogToDisplay, setError, handleSelectAccount, selectedAccount } =
    useContext(AccountsContext);
  const [accountToSwitchTo, setAccountToSwitchTo] = useState<string>();

  const theme = useTheme();

  const handleClose = () => {
    setDialogToDisplay(undefined);
    setError(undefined);
    setAccountToSwitchTo(undefined);
  };

  if (accountToSwitchTo)
    return (
      <ConfirmPasswordModal
        modalTitle="Switch account"
        accountName={accountToSwitchTo}
        buttonTitle="Switch account"
        onClose={() => {
          handleClose();
          setDialogToDisplay('Accounts');
        }}
        onConfirm={async (password) => {
          const isSuccessful = await handleSelectAccount({ password, accountName: accountToSwitchTo });
          if (isSuccessful) handleClose();
        }}
      />
    );

  return (
    <Dialog
      open={dialogToDisplay === 'Accounts'}
      onClose={handleClose}
      fullWidth
      maxWidth="sm"
      PaperProps={{
        elevation: 4,
        sx: {
          borderRadius: 2,
          overflow: 'hidden',
          border: `1px solid ${theme.palette.nym.nymWallet.modal.border}`,
          display: 'flex',
          flexDirection: 'column',
          maxHeight: '80vh', // Limit maximum height
          ...(theme.palette.mode === 'dark' && {
            backgroundImage: 'linear-gradient(180deg, rgba(50, 55, 61, 0.8), rgba(36, 43, 45, 0.95))',
          }),
        },
      }}
    >
      <DialogTitle sx={{ pb: 1, flexShrink: 0 }}>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Box display="flex" alignItems="center" gap={1}>
            <SwapHorizOutlined
              sx={{
                color: theme.palette.nym.highlight,
                backgroundColor: alpha(theme.palette.nym.highlight, 0.1),
                borderRadius: '50%',
                p: 0.5,
                fontSize: 24,
              }}
            />
            <Typography variant="h6" fontWeight={600}>
              Accounts
            </Typography>
          </Box>
          <IconButton
            onClick={handleClose}
            size="small"
            sx={{
              backgroundColor: alpha(theme.palette.text.primary, 0.05),
              '&:hover': {
                backgroundColor: alpha(theme.palette.text.primary, 0.1),
              },
              width: 30,
              height: 30,
            }}
          >
            <Close fontSize="small" />
          </IconButton>
        </Box>
        <Typography
          variant="body2"
          sx={{
            color:
              theme.palette.mode === 'dark'
                ? theme.palette.nym.nymWallet.text.muted
                : alpha(theme.palette.text.primary, 0.6),
            pl: 4.5,
          }}
        >
          Switch between accounts
        </Typography>
      </DialogTitle>

      <DialogContent
        sx={{
          px: 1,
          pt: 0,
          flexGrow: 1,
          overflowY: 'auto', // Enable vertical scrolling
          minHeight: '100px', // Ensure minimum height for content
          '&::-webkit-scrollbar': {
            width: '8px',
            height: '8px',
          },
          '&::-webkit-scrollbar-thumb': {
            backgroundColor:
              theme.palette.mode === 'dark'
                ? alpha(theme.palette.nym.nymWallet.background.greyStroke, 0.8)
                : alpha(theme.palette.nym.nymWallet.background.greyStroke, 0.5),
            borderRadius: '4px',
          },
          '&::-webkit-scrollbar-track': {
            backgroundColor: 'transparent',
          },
        }}
      >
        <List sx={{ py: 1 }}>
          {accounts?.map(({ id, address }) => (
            <AccountItem
              name={id}
              address={address}
              key={address}
              onSelectAccount={() => {
                if (selectedAccount?.id !== id) {
                  setAccountToSwitchTo(id);
                }
              }}
            />
          ))}
        </List>
      </DialogContent>

      <Box sx={{ flexShrink: 0 }}>
        <Divider
          variant="middle"
          sx={{
            my: 1.5,
            opacity: 0.6,
          }}
        />

        <DialogActions
          sx={{
            p: 3,
            justifyContent: 'space-between',
          }}
        >
          <Button
            startIcon={<ArrowDownwardSharp />}
            onClick={() => setDialogToDisplay('Import')}
            sx={{
              borderRadius: 1.5,
              transition: 'all 0.2s',
              px: 2,
              py: 0.75,
              color: theme.palette.text.primary,
              '&:hover': {
                backgroundColor: alpha(theme.palette.text.primary, 0.05),
              },
            }}
          >
            Import account
          </Button>
          <Button
            disableElevation
            variant="contained"
            startIcon={<Add fontSize="medium" />}
            onClick={() => setDialogToDisplay('Add')}
            sx={{
              px: 2,
              py: 0.75,
              borderRadius: 1.5,
              background: theme.palette.nym.nymWallet.gradients.primary || theme.palette.nym.highlight,
              fontWeight: 600,
              boxShadow: 'none',
              transition: 'all 0.2s',
              '&:hover': {
                boxShadow:
                  theme.palette.mode === 'dark' ? '0 4px 12px rgba(0, 0, 0, 0.2)' : '0 4px 12px rgba(0, 0, 0, 0.1)',
                transform: 'translateY(-1px)',
              },
            }}
          >
            Create account
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
};
