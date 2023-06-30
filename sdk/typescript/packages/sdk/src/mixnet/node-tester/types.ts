export interface INodeTesterWorkerAsync {
  startTest: (mixnodeIdentityKey: string) => Promise<NodeTestResultResponse | undefined>;
}

export interface INodeTesterWorkerDisposableAsync {
  terminate: () => Promise<void>;
}

export interface NodeTester extends INodeTesterWorkerDisposableAsync {
  tester: INodeTesterWorkerAsync;
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

export type NodeTestResultResponse = {
  score: number;
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
    result: NodeTestResultResponse;
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
