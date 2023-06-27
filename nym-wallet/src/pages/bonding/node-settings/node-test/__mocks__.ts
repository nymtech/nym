import { WasmGateway, WasmMixNode, WasmNymTopology } from '@nymproject/nym-client-wasm';

export const createDummyTopology = () => {
  const l1Mixnode = new WasmMixNode(
    1,
    'n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47',
    '178.79.143.65',
    1789,
    '4Yr4qmEHd9sgsuQ83191FR2hD88RfsbMmB4tzhhZWriz',
    '8ndjk5oZ6HxUZNScLJJ7hk39XtUqGexdKgW7hSX6kpWG',
    1,
    '1.10.0',
  );

  const l2Mixnode = new WasmMixNode(
    2,
    'n1z93z44vf8ssvdhujjvxcj4rd5e3lz0l60wdk70',
    '109.74.197.180',
    1789,
    '7sVjiMrPYZrDWRujku9QLxgE8noT7NTgBAqizCsu7AoK',
    'GepXwRnKZDd8x2nBWAajGGBVvF3mrpVMQBkgfrGuqRCN',
    2,
    '1.10.0',
  );

  const l3Mixnode = new WasmMixNode(
    3,
    'n1ptg680vnmef2cd8l0s9uyc4f0hgf3x8sed6w77',
    '176.58.101.80',
    1789,
    'FoM5Mx9Pxk1g3zEqkS3APgtBeTtTo3M8k7Yu4bV6kK1R',
    'DeYjrDC2AcQRVFshiKnbUo6bRvPyZ33QGYR2DLeFJ9qD',
    3,
    '1.10.0',
  );

  const gateway = new WasmGateway(
    'n16evnn8glr0sham3matj8rg2s24m6x56ayk87ts',
    '85.159.212.96',
    1789,
    9000,
    '336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9',
    'BtYjoWihiuFihGKQypmpSspbhmWDPxzqeTVSd8ciCpWL',
    '1.10.1',
  );

  const mixnodes = new Map();
  mixnodes.set(1, [l1Mixnode]);
  mixnodes.set(2, [l2Mixnode]);
  mixnodes.set(3, [l3Mixnode]);

  const gateways = [gateway];

  return new WasmNymTopology(mixnodes, gateways);
};
