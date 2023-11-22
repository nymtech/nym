import React, { useEffect, useState } from 'react';
import { useChain } from '@cosmos-kit/react';
import { Box, Button, Typography, IconButton, Stack } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import { TokenSVG } from '../icons/TokenSVG';
import { ElipsSVG } from '../icons/ElipsSVG';
import { trimAddress } from '../utils';
import { unymToNym } from '../utils/currency';
import { useIsMobile } from '../hooks/useIsMobile';

export const ConnectKeplrWallet = () => {
  const { connect, disconnect, wallet, address, getCosmWasmClient, isWalletConnected, isWalletConnecting } =
    useChain('nyx');
  const isMobile = useIsMobile(1200);

  const [balance, setBalance] = useState<{
    status: 'loading' | 'success';
    data?: string;
  }>({ status: 'loading', data: undefined });

  useEffect(() => {
    const getBalance = async (walletAddress: string) => {
      setBalance({ status: 'loading', data: undefined });

      const account = await getCosmWasmClient();
      const uNYMBalance = await account.getBalance(walletAddress, 'unym');
      const NYMBalance = unymToNym(uNYMBalance.amount);

      setBalance({ status: 'success', data: NYMBalance });
    };

    if (address) {
      getBalance(address);
    }
  }, [address, getCosmWasmClient]);

  const getGlobalbutton = () => {
    if (isWalletConnecting) {
      return <Button onClick={() => connect()}>{`Connecting ${wallet?.prettyName}`}</Button>;
    }
    if (isWalletConnected) {
      return (
        <Stack direction="row" spacing={1}>
          {!isMobile && (
            <Box display="flex" alignItems="center" gap={1}>
              <TokenSVG />
              <Typography variant="body1" fontWeight={600}>
                {balance.data} NYM
              </Typography>
            </Box>
          )}{' '}
          <Box display="flex" alignItems="center" gap={1}>
            <ElipsSVG />
            <Typography variant="body1" fontWeight={600}>
              {trimAddress(address, 7)}
            </Typography>
          </Box>
          <IconButton
            size="small"
            onClick={async () => {
              await disconnect();
            }}
          >
            <CloseIcon fontSize="small" sx={{ color: 'white' }} />
          </IconButton>
        </Stack>
      );
    }

    return <Button onClick={() => connect()}>Connect Wallet</Button>;
  };

  return <Box sx={{ mr: 2 }}>{getGlobalbutton()}</Box>;
};
