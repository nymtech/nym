import React from 'react';
import { useChain, useWalletClient } from '@cosmos-kit/react';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import { pubkeyType } from '@cosmjs/amino';
import { toBase64 } from '@cosmjs/encoding';
import { Alert, AlertTitle, LinearProgress } from '@mui/material';
import { aminoDoc } from './data';

export const CosmosKitLedger = () => {
  const { wallet, address } = useChain('nyx');
  const { client } = useWalletClient(wallet?.name);
  const [signResponse, setSignResponse] = React.useState<any>();
  const [busy, setBusy] = React.useState<boolean>();

  const sign = async () => {
    setBusy(true);

    const serialized = aminoDoc(address);
    const account = await client.getAccount('nyx');

    console.log('Accounts: ', account);
    console.log('Info', await (client as any).client.getAppConfiguration());
    const sigAmino = await (client as any).client.sign(account.username, serialized);
    const sig = {
      signature: toBase64(sigAmino.signature),
      pub_key: {
        type: pubkeyType.secp256k1,
        value: toBase64(account.pubkey),
      },
    };
    console.log('Sig', { sigAmino, sig });
    setBusy(false);

    return { signature: sig };
  };

  const handleSign = async () => {
    setSignResponse(await sign());
  };

  if (busy) {
    return (
      <Box mt={4} mb={2}>
        <LinearProgress color="success" />
        <Alert severity="success">
          <AlertTitle>Please approve on the Ledger</AlertTitle>
          Follow the instructions on the Ledger to review the message
        </Alert>
      </Box>
    );
  }

  return (
    <>
      {!signResponse && (
        <Box mt={4} mb={2}>
          <Box mb={2}>Click the button below to sign a fake request with your Ledger</Box>
          <Button variant="outlined" onClick={handleSign}>
            Click to sign
          </Button>
        </Box>
      )}
      {signResponse && (
        <Box mt={2}>
          <strong>Signature:</strong>
          <Box sx={{ overflowX: 'auto' }}>
            <pre>{JSON.stringify(signResponse?.signature, null, 2)}</pre>
          </Box>
        </Box>
      )}
    </>
  );
};
