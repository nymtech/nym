import React from 'react';
import { ChainProvider, useChain } from '@cosmos-kit/react';
import { assets, chains } from 'chain-registry';
import { wallets as keplr } from '@cosmos-kit/keplr';
import { wallets as ledger } from '@cosmos-kit/ledger';
import Button from '@mui/material/Button';
import CircularProgress from '@mui/material/CircularProgress';
import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import { SignDoc } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import { AminoMsg, makeSignDoc } from '@cosmjs/amino';
import { fromHex } from '@cosmjs/encoding';
import { Alert } from '@mui/material';
import { MsgSend } from 'cosmjs-types/cosmos/bank/v1beta1/tx';

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
    <ChainProvider chains={chainsFixedUp} assetLists={assetsFixedUp} wallets={[...ledger, ...keplr]}>
      {children}
    </ChainProvider>
  );
};

const CosmosKitInner = () => {
  const {
    wallet,
    address,
    connect,
    disconnect,
    getSigningStargateClient,
    getOfflineSigner,
    getOfflineSignerDirect,
    getOfflineSignerAmino,
    signAmino,
    signArbitrary,
    isWalletConnecting,
    isWalletDisconnected,
    isWalletError,
    isWalletNotExist,
    isWalletRejected,
  } = useChain('nyx');
  const [signResponse, setSignResponse] = React.useState<any>();

  const sign = async () => {
    // from https://github.com/cosmos/cosmjs/blob/main/packages/proto-signing/src/testutils.spec.ts#L18C13-L25C6
    const bodyBytes = fromHex(
      '0a90010a1c2f636f736d6f732e62616e6b2e763162657461312e4d736753656e6412700a2d636f736d6f7331706b707472653766646b6c366766727a6c65736a6a766878686c63337234676d6d6b38727336122d636f736d6f7331717970717870713971637273737a673270767871367273307a716733797963356c7a763778751a100a0575636f736d120731323334353637',
    );
    const authInfoBytes = fromHex(
      '0a4e0a460a1f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657912230a21034f04181eeba35391b858633a765c4a0c189697b40d216354d50890d350c7029012040a02080112130a0d0a0575636f736d12043230303010c09a0c',
    );

    if (wallet.mode === 'ledger') {
      console.log('Using ledger to sign...');
      const chainId = 'nyx';
      // const msg: AminoMsg = {
      //   type: 'cosmos-sdk/MsgSend',
      //   value: {
      //     from_address: address,
      //     to_address: 'cosmos1pkptre7fdkl6gfrzlesjjvhxhlc3r4gmmk8rs6',
      //     amount: [{ amount: '1234567', denom: 'ucosm' }],
      //   },
      // };
      const msg = {
        typeUrl: '/cosmos.bank.v1beta1.MsgSend',
        value: MsgSend.fromPartial({
          fromAddress: address,
          toAddress: 'cosmos1pkptre7fdkl6gfrzlesjjvhxhlc3r4gmmk8rs6',
          amount: [{ amount: '1234567', denom: 'ucosm' }],
        }),
      };
      const fee = {
        amount: [{ amount: '2000', denom: 'ucosm' }],
        gas: '180000', // 180k
      };
      const memo = 'Use your power wisely';
      // const accountNumber = 15;
      // const sequence = 16;

      // const signDoc = makeSignDoc([msg], fee, chainId, memo, accountNumber, sequence);
      // return getOfflineSignerAmino().signAmino(address, signDoc);
      // return signAmino(address, signDoc);

      const tx = await (await getSigningStargateClient()).sign(address, [msg], fee, memo);
      return tx;
      // return signArbitrary(address, 'hello world');
    }

    const doc = SignDoc.fromPartial({ accountNumber: address, chainId: 'nyx', bodyBytes, authInfoBytes });
    return getOfflineSignerDirect().signDirect(address, doc);
  };

  const handleSign = async () => {
    setSignResponse(await sign());
  };

  if (isWalletError) {
    return (
      <Box mt={4} mb={2}>
        <Alert severity="error">Oh no! Something went wrong.</Alert>
        <Box mt={4}>
          <Button variant="outlined" onClick={disconnect}>
            Disconnect
          </Button>
        </Box>
      </Box>
    );
  }

  if (isWalletNotExist) {
    return (
      <Box mt={4} mb={2}>
        <Alert severity="error">Oh no! Could not connect to that wallet.</Alert>
        <Box mt={4}>
          <Button variant="outlined" onClick={disconnect}>
            Disconnect
          </Button>
        </Box>
      </Box>
    );
  }

  if (isWalletRejected) {
    return (
      <Box mt={4} mb={2}>
        <Alert severity="error">Oh no! Did you authorize the connection to your wallet?</Alert>
        <Box mt={4}>
          <Button variant="outlined" onClick={disconnect}>
            Disconnect
          </Button>
        </Box>
      </Box>
    );
  }

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
          <strong>
            Connected to {wallet.prettyName} ({wallet.name})
          </strong>
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
