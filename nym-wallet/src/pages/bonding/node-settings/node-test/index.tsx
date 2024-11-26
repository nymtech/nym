import React, { useCallback, useContext, useEffect, useRef, useState } from 'react';
import { Box, Button } from '@mui/material';
import { Download } from '@mui/icons-material';
import { NodeTestResultResponse, NodeTester, createNodeTesterClient } from '@nymproject/node-tester';
import { format } from 'date-fns';
import { AppContext, useBondingContext } from 'src/context';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { Results } from 'src/components/TestNode/Results';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { PrintResults } from 'src/components/TestNode/PrintResults';
import { MAINNET_VALIDATOR_URL, QA_VALIDATOR_URL } from 'src/constants';
import { TestStatus } from 'src/components/TestNode/types';
import { isMixnode } from 'src/types';

export const NodeTestPage = () => {
  const [nodeTestClient, setNodeTestClient] = useState<NodeTester>();
  const [error, setError] = useState<string>();
  const [isLoading, setIsLoading] = useState(false);
  const [results, setResults] = useState<NodeTestResultResponse>();
  const [printResults, setPrintResults] = useState(false);
  const [testDate, setTestDate] = useState<string>();

  const testStateRef = useRef<TestStatus>('Stopped');
  const timerRef = useRef<NodeJS.Timeout>();

  const { network } = useContext(AppContext);
  const { bondedNode } = useBondingContext();

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

  const handleTestNode = async () => {
    if (nodeTestClient && bondedNode && isMixnode(bondedNode)) {
      setResults(undefined);
      setTestDate(format(new Date(), 'dd/MM/yyyy HH:mm'));
      setIsLoading(true);
      setError(undefined);
      testStateRef.current = 'Running';
      handleTestTimeout();
      try {
        const result = await nodeTestClient.tester.startTest(bondedNode.identityKey);
        setResults(result);
        testStateRef.current = 'Complete';
      } catch (e) {
        setError('Node test failed, please try again');
        testStateRef.current = 'Stopped';
        console.log(e);
      } finally {
        setIsLoading(false);
      }
    }
  };

  const loadNodeTestClient = useCallback(async () => {
    try {
      const nodeTesterId = new Date().toISOString(); // make a new tester id for each session
      const validator = network === 'MAINNET' ? MAINNET_VALIDATOR_URL : QA_VALIDATOR_URL;
      const client = await createNodeTesterClient();
      await client.tester.init(validator, nodeTesterId);
      setNodeTestClient(client);
    } catch (e) {
      console.log(e);
      setError('Failed to load node tester client, please try again');
    }
  }, []);

  useEffect(() => {
    loadNodeTestClient();

    return () => {
      clearTimeout(timerRef.current);
      if (nodeTestClient) {
        nodeTestClient.tester.disconnectFromGateway();
        nodeTestClient.terminate();
      }
    };
  }, [loadNodeTestClient]);

  return (
    <Box p={4}>
      {isLoading && <LoadingModal text="Testing mixnode, please wait.." />}
      {error && <ErrorModal open title="Node test failed" message={error} onClose={() => setError(undefined)} />}
      {printResults && results && bondedNode && isMixnode(bondedNode) && (
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
