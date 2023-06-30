export interface INodeTesterWorkerAsync {
  startTest: (mixnodeId: string) => Promise<NodeTestResult | undefined>;
}

export interface INodeTesterWorkerDisposableAsync {
  terminate: () => Promise<void>;
}

export interface INodeTesterWorker {
  startTest: (mixnodeId: string) => Promise<NodeTestResult | undefined>;
}

export interface NodeTester extends INodeTesterWorkerDisposableAsync {
  tester: INodeTesterWorker;
}

export enum NodeTesterEventKinds {
  Loaded = 'Loaded',
  Connected = 'Connected',
}

export interface NodeTesterLoadedEvent {
  kind: NodeTesterEventKinds.Loaded;
  args: {
    loaded: true;
  };
}

export type Network = 'QA' | 'SANDBOX' | 'MAINNET';

export type NodeTestResult = {
  score: number;
  sentPackets: number;
  receivedPackets: number;
  receivedAcks: number;
  duplicatePackets: number;
  duplicateAcks: number;
};

export type Error = {
  kind: 'Error';
  args: { message: string };
};

export type WorkerLoaded = {
  kind: 'WorkerLoaded';
};

export type DisplayTesterResults = {
  kind: 'DisplayTesterResults';
  args: {
    result: NodeTestResult;
  };
};

export type TestPacket = {
  kind: 'TestPacket';
  args: {
    mixnodeIdentity: string;
    network: Network;
  };
};

export type TestStatus = 'Stopped' | 'Running' | 'Complete';

export type NodeTestEvent = Error | DisplayTesterResults | TestPacket | WorkerLoaded;
