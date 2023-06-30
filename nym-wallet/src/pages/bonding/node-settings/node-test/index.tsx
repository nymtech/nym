import React, { useContext, useEffect, useRef, useState } from 'react';
import { Box, Button } from '@mui/material';
import { AppContext, useBondingContext } from 'src/context';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { Results } from 'src/components/TestNode/Results';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { PrintResults } from 'src/components/TestNode/PrintResults';
import { Download } from '@mui/icons-material';
import { format } from 'date-fns';
import { createNodeTesterClient } from '@nymproject/sdk';
import { NodeTestEvent, NodeTestResult, TestStatus } from './types';

export const NodeTestPage = () => {
  const [nodeTestWorker, setNodeTestWorker] = useState<Worker>();
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);
  const [results, setResults] = useState<NodeTestResult>();
  const [printResults, setPrintResults] = useState(false);
  const [testDate, setTestDate] = useState<string>();

  const testStateRef = useRef<TestStatus>('Stopped');
  const timerRef = useRef<NodeJS.Timeout>();

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

  const handleTestTimeout = () => {
    clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      if (testStateRef.current === 'Running') {
        setIsLoading(false);
        setError('Test has timed out, please try again');
        testStateRef.current = 'Stopped';
      }
    }, 15000);
  };

  const handleWorkerMessages = (ev: MessageEvent<NodeTestEvent>) => {
    const eventKind = ev.data.kind;

    if (eventKind === 'Error') {
      setError(ev.data.args.message);
      testStateRef.current = 'Stopped';
    }
    if (eventKind === 'DisplayTesterResults') {
      setResults(ev.data.args.result);
      testStateRef.current = 'Complete';
    }
    setIsLoading(false);
  };

  const handleTestNode = async () => {
    setError(undefined);
    setResults(undefined);
    setIsLoading(true);
    setTestDate(format(new Date(), 'dd/MM/yyyy HH:mm'));

    if (nodeTestWorker) {
      testStateRef.current = 'Running';
      nodeTestWorker.postMessage({
        kind: 'TestPacket',
        args: {
          mixnodeIdentity: bondedNode?.identityKey,
          network,
        },
      } as NodeTestEvent);
      handleTestTimeout();
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
      nodeTestWorker.addEventListener('message', (e) => handleWorkerMessages(e));
    }

    return () => nodeTestWorker?.removeEventListener('message', handleWorkerMessages);
  }, [nodeTestWorker]);

  console.log(createNodeTesterClient);

  return (
    <Box p={4}>
      {isLoading && <LoadingModal text="Testing mixnode, please wait.." />}
      {error && <ErrorModal open title="Node test failed" message={error} onClose={() => setError(undefined)} />}
      {printResults && results && (
        <PrintResults
          mixnodeId={bondedNode?.identityKey || '-'}
          mixnodeName={bondedNode?.name || '-'}
          packetsSent={results.sentPackets}
          packetsReceived={results.receivedPackets}
          score={results.score}
          date={testDate}
          OnPrintRequestComplete={() => setPrintResults(false)}
        />
      )}
      <Results
        packetsSent={results?.sentPackets}
        packetsReceived={results?.receivedPackets}
        score={results?.score}
        status={testStateRef.current}
        date={testDate}
        onStartTest={handleTestNode}
      />
      <Box display="flex" justifyContent="flex-end">
        <Button onClick={() => setPrintResults(true)} startIcon={<Download />} disabled={!results}>
          Save test results as PDF
        </Button>
      </Box>
    </Box>
  );
};
