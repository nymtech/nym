import { Network } from 'src/types';

export type NodeTestResult = {
  score: number;
  sentPackets: number;
  receivedPackets: number;
  receivedAcks: number;
  duplicatePackets: number;
  duplicateAcks: number;
};

type Error = {
  kind: 'Error';
  args: { message: string };
};

type WorkerLoaded = {
  kind: 'WorkerLoaded';
};

type DisplayTesterResults = {
  kind: 'DisplayTesterResults';
  args: {
    result: NodeTestResult;
  };
};

type TestPacket = {
  kind: 'TestPacket';
  args: {
    mixnodeIdentity: string;
    network: Network;
  };
};

export type TestStatus = 'Stopped' | 'Running' | 'Complete';

export type NodeTestEvent = Error | DisplayTesterResults | TestPacket | WorkerLoaded;
