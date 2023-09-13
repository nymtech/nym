import * as React from 'react';

import { ChainProvider, useChain } from '@cosmos-kit/react';
import { chains, assets } from 'chain-registry';
import { wallets } from '@cosmos-kit/keplr';
import Button from '@mui/material/Button';
import CircularProgress from '@mui/material/CircularProgress';
import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import { SignDoc } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import { fromHex } from '@cosmjs/encoding';

const CosmosKitSetup: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const assetsFixedUp = React.useMemo(() => {
    const nyx = assets.find((a) => a.chain_name === 'nyx');
    if (nyx) {
      const nyxCoin = nyx.assets.find((a) => a.name === 'nyx');
      if (nyxCoin) {
        nyxCoin.coingecko_id = 'nyx';
      }
      nyx.assets = nyx.assets.reverse();
    }
    return assets;
  }, [assets]);

  const chainsFixedUp = React.useMemo(() => {
    const nyx = chains.find((c) => c.chain_id === 'nyx');
    if (nyx) {
      if (!nyx.staking) {
        nyx.staking = {
          staking_tokens: [{ denom: 'unyx' }],
          lock_duration: {
            blocks: 10000,
          },
        };
      }
    }
    return chains;
  }, [chains]);

  return (
    <ChainProvider chains={chainsFixedUp} assetLists={assetsFixedUp} wallets={wallets}>
      {children}
    </ChainProvider>
  );
};

const CosmosKitInner = () => {
  const { wallet, address, connect, disconnect, getOfflineSignerDirect, isWalletConnecting, isWalletDisconnected } =
    useChain('nyx');
  const [signResponse, setSignResponse] = React.useState<any>();

  const sign = async () => {
    // from https://github.com/cosmos/cosmjs/blob/main/packages/proto-signing/src/testutils.spec.ts#L18C13-L25C6
    const bodyBytes = fromHex(
      '0a90010a1c2f636f736d6f732e62616e6b2e763162657461312e4d736753656e6412700a2d636f736d6f7331706b707472653766646b6c366766727a6c65736a6a766878686c63337234676d6d6b38727336122d636f736d6f7331717970717870713971637273737a673270767871367273307a716733797963356c7a763778751a100a0575636f736d120731323334353637',
    );
    const authInfoBytes = fromHex(
      '0a4e0a460a1f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657912230a21034f04181eeba35391b858633a765c4a0c189697b40d216354d50890d350c7029012040a02080112130a0d0a0575636f736d12043230303010c09a0c',
    );

    const doc = SignDoc.fromPartial({ accountNumber: address, chainId: 'nyx', bodyBytes, authInfoBytes });
    return getOfflineSignerDirect().signDirect(address, doc);
  };

  const handleSign = async () => {
    setSignResponse(await sign());
  };

  if (isWalletDisconnected) {
    return (
      <Button variant="outlined" onClick={connect} sx={{ mt: 4 }}>
        Connect Keplr
      </Button>
    );
  }

  if (isWalletConnecting) {
    return <CircularProgress />;
  }

  return (
    <Paper sx={{ mt: 4, py: 4, px: 2 }} elevation={2}>
      <Box display="flex" justifyContent="space-between">
        <Box>
          <strong>Connected to {wallet.prettyName}</strong>
          <Typography>
            Address: <code>{address}</code>{' '}
          </Typography>
        </Box>
        <Box>
          <Button variant="outlined" onClick={disconnect}>
            Disconnect
          </Button>
        </Box>
      </Box>

      {!signResponse && (
        <Box mt={4} mb={2}>
          <Box mb={2}>Click the button below to sign a fake request with Keplr</Box>
          <Button variant="outlined" onClick={handleSign}>
            Click to sign
          </Button>
        </Box>
      )}
      {signResponse && (
        <Box mt={2}>
          <strong>Signature:</strong>
          <Box sx={{ overflowX: 'auto' }}>
            <pre>{JSON.stringify(signResponse.signature, null, 2)}</pre>
          </Box>
        </Box>
      )}
    </Paper>
  );
};

export const CosmosKit = () => (
  <CosmosKitSetup>
    <CosmosKitInner />
  </CosmosKitSetup>
);
