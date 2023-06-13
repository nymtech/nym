import React, { useEffect, useState } from 'react';
import { Button, Stack, Typography } from '@mui/material';

export const NodeTester = () => {
  const [nodeTestWorker, setNodeTestWorker] = useState<Worker>();
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);

  const loadWorker = () => {
    try {
      const worker: Worker = new Worker(new URL('./worker.ts', import.meta.url));
      setNodeTestWorker(worker);
    } catch (e) {
      setError('Error loading worker');
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

  const handleWorkerMessages = (worker: Worker) => {
    worker.onmessage = (ev) => {
      const messageKind = ev?.data?.kind;

      if (messageKind === 'Error') {
        setError(ev.data.args.message);
      }
      if (messageKind === 'DisplayTesterResults') {
        console.log(ev.data);
      }
    };
    setIsLoading(false);
  };

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
          mixnodeIdentity: '7sVjiMrPYZrDWRujku9QLxgE8noT7NTgBAqizCsu7AoK',
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
