export interface IWorkerAsync {
  startTest: (mixnodeId: string) => Promise<NodeTestResult | undefined>;
}

export interface IWorker {
  startTest: (mixnodeId: string) => NodeTestResult | undefined;
}

export enum EventTypes {
  Loaded = 'Loaded',
  Connected = 'Connected',
}

type Network = 'QA' | 'SANDBOX' | 'MAINNET';

type NodeTestResult = {
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

type TestStatus = 'Stopped' | 'Running' | 'Complete';

type NodeTestEvent = Error | DisplayTesterResults | TestPacket | WorkerLoaded;

export { Network, NodeTestResult, Error, WorkerLoaded, DisplayTesterResults, TestPacket, TestStatus, NodeTestEvent };
