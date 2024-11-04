import React from 'react';
import { useChain } from '@cosmos-kit/react';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import { Alert, AlertTitle, LinearProgress } from '@mui/material';
import { getDoc } from './data';

export const CosmosKitSign = () => {
  const { address, getOfflineSignerAmino } = useChain('nyx');
  const [signResponse, setSignResponse] = React.useState<any>();
  const [busy, setBusy] = React.useState<boolean>();

  const sign = async () => {
    setBusy(true);
    const doc = getDoc(address);
    const tx = await getOfflineSignerAmino().signAmino(address, doc);
    setBusy(false);
    return tx;
  };

  const handleSign = async () => {
    setSignResponse(await sign());
  };

  if (busy) {
    return (
      <Box mt={4} mb={2}>
        <LinearProgress color="success" />
        <Alert severity="success">
          <AlertTitle>Please approve in your wallet</AlertTitle>
          Review the message to sign
        </Alert>
      </Box>
    );
  }

  return (
    <>
      {!signResponse && (
        <Box mt={4} mb={2}>
          <Box mb={2}>Click the button below to sign a fake request with your wallet</Box>
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
