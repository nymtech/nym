import React, { useEffect, useState } from 'react';
import { createNymMixnetClient, NymMixnetClient, Payload } from '@nymproject/sdk-full-fat';
import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
// download full-fat SDK to avoid worker file error from: https://www.npmjs.com/package/@nymproject/sdk-full-fat

const nymApiUrl = 'https://validator.nymtech.net/api';

export const Traffic = () => {
  const [nym, setNym] = useState<NymMixnetClient>();
  const [selfAddress, setSelfAddress] = useState<string>();
  const [recipient, setRecipient] = useState<string>();
  const [payload, setPayload] = useState<Payload>();
  const [receivedMessage, setReceivedMessage] = useState<string>();

  const init = async () => {
    const client = await createNymMixnetClient();
    setNym(client);

    // start the client and connect to a gateway
    await client?.client.start({
      clientId: crypto.randomUUID(),
      nymApiUrl,
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

  const send = () => nym.client.send({ payload, recipient });

  useEffect(() => {
    init();
    return () => {
      stop();
    };
  }, []);

  if (!nym || !selfAddress) {
    return (
      <Box sx={{ display: 'flex' }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <div>
      <h1>Use this tool to experiment with the Mixnet: send and receive messages!</h1>
      <p style={{ border: '1px solid black' }}>My self address is: {selfAddress || 'loading'}</p>
      <div style={{ border: '1px solid black' }}>
        {/* eslint-disable-next-line jsx-a11y/label-has-associated-control */}
        <label>Recipient Address</label>
        <input type="text" onChange={(e) => setRecipient(e.target.value)} />
        <input type="text" onChange={(e) => setPayload({ message: e.target.value, mimeType: 'text/plain' })} />
        <button type="button" onClick={() => send()}>
          Send
        </button>
      </div>
      <p>Received message: {receivedMessage}</p>
    </div>
  );
};
