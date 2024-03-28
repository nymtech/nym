import React from 'react';
import { Button, IconButton, Stack, CircularProgress } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import { useIsMobile } from '@src/hooks/useIsMobile';
import { useWalletContext } from '@src/context/wallet';
import { WalletAddress, WalletBalance } from '@src/components/Wallet';

export const ConnectKeplrWallet = () => {
  const { connectWallet, disconnectWallet, isWalletConnected, isWalletConnecting } = useWalletContext();
  const isMobile = useIsMobile(1200);

  if (!connectWallet || !disconnectWallet) {
    return null;
  }

  if (isWalletConnected) {
    return (
      <Stack direction="row" spacing={1}>
        <WalletBalance />
        <WalletAddress />
        <IconButton
          size="small"
          onClick={async () => {
            await disconnectWallet();
          }}
        >
          <CloseIcon fontSize="small" sx={{ color: 'white' }} />
        </IconButton>
      </Stack>
    );
  }

  return (
    <Button
      variant="outlined"
      onClick={() => connectWallet()}
      disabled={isWalletConnecting}
      endIcon={isWalletConnecting && <CircularProgress size={14} color="inherit" />}
    >
      Connect {isMobile ? '' : ' Wallet'}
    </Button>
  );
};
