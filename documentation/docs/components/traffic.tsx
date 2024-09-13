import React, { useEffect, useState } from 'react';
import { createNymMixnetClient, NymMixnetClient, Payload } from '@nymproject/sdk-full-fat';
import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';

const nymApiUrl = 'https://validator.nymtech.net/api';

export const Traffic = () => {
  const [nym, setNym] = useState<NymMixnetClient>();
  const [selfAddress, setSelfAddress] = useState<string>();
  const [recipient, setRecipient] = useState<string>();
  const [payload, setPayload] = useState<Payload>();
  const [receivedMessage, setReceivedMessage] = useState<string>();
  const [buttonEnabled, setButtonEnabled] = useState<boolean>(false);

  const init = async () => {
    const client = await createNymMixnetClient();
    setNym(client);

    // start the client and connect to a gateway
    await client?.client.start({
      clientId: crypto.randomUUID(),
      nymApiUrl,
      forceTls: true, // force WSS
    });

    // check when is connected and set the self address
    client?.events.subscribeToConnected((e) => {
      const { address } = e.args;
      setSelfAddress(address);
    });

    // show whether the client is ready or not
    client?.events.subscribeToLoaded((e) => {
      console.log('Client ready: ', e.args);
    });

    // show message payload content when received
    client?.events.subscribeToTextMessageReceivedEvent((e) => {
      console.log(e.args.payload);
      setReceivedMessage(e.args.payload);
    });
  };

  const stop = async () => {
    await nym?.client.stop();
  };

  const send = () => payload && recipient && nym?.client.send({ payload, recipient });

  useEffect(() => {
    init();
    return () => {
      stop();
    };
  }, []);

  useEffect(() => {
    if (recipient && payload) {
      setButtonEnabled(true);
    } else {
      setButtonEnabled(false);
    }
  }, [recipient, payload]);

  if (!nym || !selfAddress) {
    return (
      <Box sx={{ display: 'flex' }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box padding={3}>
      <Paper style={{ marginTop: '1rem', padding: '2rem' }}>
        <Stack spacing={3}>
          <Typography variant="body1">My self address is:</Typography>
          <Typography variant="body1">{selfAddress || 'loading'}</Typography>
          <Typography variant="h5">Communication through the Mixnet</Typography>
          <TextField
            type="text"
            placeholder="Recipient Address"
            onChange={(e) => setRecipient(e.target.value)}
            size="small"
          />
          <TextField
            type="text"
            placeholder="Message to send"
            multiline
            rows={4}
            onChange={(e) => setPayload({ message: e.target.value, mimeType: 'text/plain' })}
            size="small"
          />
          <Button variant="outlined" onClick={() => send()} disabled={!buttonEnabled} sx={{ width: 'fit-content' }}>
            Send
          </Button>
        </Stack>
        {receivedMessage && (
          <Stack spacing={3} style={{ marginTop: '1rem' }}>
            <Typography variant="h5">Message Received!</Typography>
            <Typography fontFamily="monospace">{receivedMessage}</Typography>
          </Stack>
        )}
      </Paper>
    </Box>
  );
};
