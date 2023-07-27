import { useState, useEffect } from 'react';
import { createNodeTesterClient, NodeTester } from '@nymproject/sdk';

export type TestState = 'Ready' | 'Connecting' | 'Disconnected' | 'Disconnecting' | 'Error' | 'Testing' | 'Stopped';

export const useNodeTesterClient = () => {
  const [client, setClient] = useState<NodeTester>();
  const [error, setError] = useState<string>();
  const [testState, setTestState] = useState<TestState>('Disconnected');

  const createClient = async () => {
    setTestState('Connecting');
    try {
      const validator = 'https://validator.nymtech.net/api';
      const nodeTesterClient = await createNodeTesterClient();

      await nodeTesterClient.tester.init(validator);
      setClient(nodeTesterClient);
    } catch (e) {
      console.log(e);
      setError('Failed to load node tester client, please try again');
    } finally {
      setTestState('Ready');
    }
  };

  useEffect(() => {
    createClient();
  }, []);

  const testNode = !client
    ? undefined
    : async (mixnodeIdentity: string) => {
        try {
          setTestState('Testing');
          const result = await client.tester.startTest(mixnodeIdentity);
          setTestState('Ready');
          return result;
        } catch (e) {
          console.log(e);
          setError('Failed to test node, please try again');
          setTestState('Error');
        }
      };

  const disconnectFromGateway = !client
    ? undefined
    : async () => {
        setTestState('Disconnecting');
        await client.tester.disconnectFromGateway();
        setTestState('Disconnected');
      };

  const reconnectToGateway = !client
    ? undefined
    : async () => {
        setTestState('Connecting');
        await client.tester.reconnectToGateway();
        setTestState('Ready');
      };

  const terminateWorker = !client
    ? undefined
    : async () => {
        setTestState('Disconnecting');
        await client.terminate();
        setTestState('Disconnected');
      };

  return { testNode, disconnectFromGateway, reconnectToGateway, terminateWorker, testState, error };
};
