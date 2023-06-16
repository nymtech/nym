import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Stack, Typography } from '@mui/material';
import { AppContext, useBondingContext } from 'src/context';
import { NodeTestEvent, NodeTestResult } from './types';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { Results } from 'src/components/TestNode/Results';
import { ErrorModal } from 'src/components/Modals/ErrorModal';

export const NodeTestPage = () => {
  const [nodeTestWorker, setNodeTestWorker] = useState<Worker>();
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);
  const [results, setResults] = useState<NodeTestResult>();

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
        setResults(ev.data.args.result);
      }
      setIsLoading(false);
    };
  };

  const handleTestNode = async () => {
    setError(undefined);
    setResults(undefined);
    setIsLoading(true);

    if (nodeTestWorker) {
      nodeTestWorker.postMessage({
        kind: 'TestPacket',
        args: {
          mixnodeIdentity: bondedNode?.identityKey,
          network,
        },
      });
    }
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

  return (
    <Box p={4}>
      {isLoading && <LoadingModal text={`Testing mixnode, please wait..`} />}
      {error && <ErrorModal open onClose={() => setError(undefined)} title="Node test failed" message={error} />}
      <Typography>{error}</Typography>
      <Results packetsSent={results?.sentPackets} packetsReceived={results?.receivedPackets} score={results?.score} />
      <Box sx={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button variant="contained" disableElevation onClick={handleTestNode} disabled={isLoading}>
          Start test
        </Button>
      </Box>
    </Box>
  );
};
