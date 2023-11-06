import React from 'react';
import { useChain } from '@cosmos-kit/react';
import { Box, Button, Card, Typography, IconButton } from '@mui/material';
import Big from 'big.js';
import CloseIcon from '@mui/icons-material/Close';
import { useTheme } from '@mui/material/styles';

import { useEffect, useState, useMemo } from 'react';

import '@interchain-ui/react/styles';
import { TokenSVG } from '../icons/TokenSVG';
import { ElipsSVG } from '../icons/ElipsSVG';

export function useIsClient() {
  const [isClient, setIsClient] = useState(false);

  useEffect(() => {
    setIsClient(true);
  }, []);

  return isClient;
}

export const uNYMtoNYM = (unym: string, rounding = 6) => {
  const nym = Big(unym).div(1000000).toFixed(rounding);

  return {
    asString: () => {
      return nym;
    },
    asNumber: () => {
      return Number(nym);
    },
  };
};

export const trimAddress = (address = '', trimBy = 6) => {
  return `${address.slice(0, trimBy)}...${address.slice(-trimBy)}`;
};

export default function ConnectKeplrWallet() {
  const { username, connect, disconnect, wallet, openView, address, getCosmWasmClient, isWalletConnected } =
    useChain('nyx');
  const isClient = useIsClient();
  const theme = useTheme();

  const color = theme.palette.text.primary;

  const [balance, setBalance] = useState<{
    status: 'loading' | 'success';
    data?: string;
  }>({ status: 'loading', data: undefined });

  useEffect(() => {
    const getBalance = async (walletAddress: string) => {
      setBalance({ status: 'loading', data: undefined });

      const account = await getCosmWasmClient();
      const uNYMBalance = await account.getBalance(walletAddress, 'unym');
      const NYMBalance = uNYMtoNYM(uNYMBalance.amount).asString();

      setBalance({ status: 'success', data: NYMBalance });
    };

    if (address) {
      getBalance(address);
    }
  }, [address, getCosmWasmClient]);

  if (!isClient) return null;

  const getGlobalbutton = () => {
    // if (globalStatus === 'Connecting') {
    //   return <Button onClick={() => connect()}>{`Connecting ${wallet?.prettyName}`}</Button>;
    // }
    if (isWalletConnected) {
      return (
        <Box display={'flex'} alignItems={'center'} gap={2}>
          <Box display={'flex'} alignItems={'center'} gap={1}>
            <TokenSVG />
            <Typography variant="body1" fontWeight={600}>
              {balance.data} NYM
            </Typography>
          </Box>
          <Box display={'flex'} alignItems={'center'} gap={1}>
            <ElipsSVG />
            <Typography variant="body1" fontWeight={600}>
              {trimAddress(address, 7)}
            </Typography>
          </Box>
          <IconButton
            onClick={async () => {
              await disconnect();
              // setGlobalStatus(WalletStatus.Disconnected);
            }}
          >
            <CloseIcon sx={{ color: 'white' }} />
          </IconButton>
        </Box>
      );
    }

    return <Button onClick={() => connect()}>Connect Wallet</Button>;
  };

  return (
    <Box>
      <div className="flex justify-start space-x-5">{getGlobalbutton()}</div>
    </Box>
  );
}
