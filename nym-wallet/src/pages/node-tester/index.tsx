import React, { useContext, useEffect, useState } from 'react';
import { Button, Stack, Typography } from '@mui/material';
import { AppContext, BondingContext, BondingContextProvider, useBondingContext } from 'src/context';
import { NodeTestEvent } from './types';

const NodeTester = () => {
  const [nodeTestWorker, setNodeTestWorker] = useState<Worker>();
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);

  const { network } = useContext(AppContext);
  const { bondedNode } = useBondingContext();

  const loadWorker = () => {
    try {
      const worker: Worker = new Worker(new URL('./worker.ts', import.meta.url));
      setNodeTestWorker(worker);
    } catch (e) {
      setError('Error loading worker');
    }
  };

  const handleWorkerMessages = (worker: Worker) => {
    worker.onmessage = (ev: MessageEvent<NodeTestEvent>) => {
      const eventKind = ev.data.kind;

      if (eventKind === 'Error') {
        setError(ev.data.args.message);
      }
      if (eventKind === 'DisplayTesterResults') {
        console.log(ev.data.args.data);
      }
    };
    setIsLoading(false);
  };

  useEffect(() => {
    loadWorker();

    return () => {
      if (nodeTestWorker) {
        nodeTestWorker.terminate();
      }
    };
  }, []);

  useEffect(() => {
    if (nodeTestWorker) {
      handleWorkerMessages(nodeTestWorker);
    }
  }, [nodeTestWorker]);

  const handleTestNode = async () => {
    if (nodeTestWorker) {
      setError(undefined);
      setIsLoading(true);

      nodeTestWorker.postMessage({
        kind: 'TestPacket',
        args: {
          mixnodeIdentity: bondedNode?.identityKey,
          network,
        },
      });
    }
  };

  return (
    <Stack>
      <h1>Test Node</h1>
      <Button variant="contained" disableElevation onClick={handleTestNode} disabled={isLoading}>
        Test
      </Button>
      <Typography>{error}</Typography>
    </Stack>
  );
};

export const NodeTestPage = () => (
  <BondingContextProvider>
    <NodeTester />
  </BondingContextProvider>
);
