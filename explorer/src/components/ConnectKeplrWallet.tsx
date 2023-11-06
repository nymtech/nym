import React from 'react';
import { useChain } from '@cosmos-kit/react';
import { Box, Button, Card } from '@mui/material';
import Big from 'big.js';

import { useEffect, useState, useMemo } from 'react';

import '@interchain-ui/react/styles';
import { TokenSVG } from '../icons/TokenSVG';

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

export default function ConnectKeplrWallet() {
  const { username, connect, disconnect, wallet, openView, address, getCosmWasmClient, isWalletConnected } =
    useChain('nyx');
  const isClient = useIsClient();

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
        <Box display={'flex'} alignItems={'center'}>
          <Button onClick={() => openView()}>
            <div>
              <span>Connected to: {wallet?.prettyName}</span>
            </div>
          </Button>

          <Box>{address}</Box>
          <TokenSVG />
          <Box> {balance.data} NYM</Box>

          <Button
            onClick={async () => {
              await disconnect();
              // setGlobalStatus(WalletStatus.Disconnected);
            }}
          >
            Disconnect
          </Button>
        </Box>
      );
    }

    return <Button onClick={() => connect()}>Connect Wallet</Button>;
  };

  return (
    <Card className="min-w-[350px] max-w-[800px] mt-20 mx-auto p-10">
      <Box>
        <div className="flex justify-start space-x-5">{getGlobalbutton()}</div>
      </Box>
    </Card>
  );
}
