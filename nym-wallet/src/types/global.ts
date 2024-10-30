import { DecCoin, Gateway, MixNode, NodeCostParams, NymNode, PledgeData } from '@nymproject/types';
import { Fee } from '@nymproject/types/dist/types/rust/Fee';
import { TBondedNode } from 'src/context';
import { TBondedGateway } from 'src/requests/gatewayDetails';
import { TBondedMixnode } from 'src/requests/mixnodeDetails';
import { TBondedNymNode } from 'src/requests/nymNodeDetails';

export enum EnumNodeType {
  mixnode = 'mixnode',
  gateway = 'gateway',
  nymnode = 'nymnode',
}

export type TNodeOwnership = {
  hasOwnership: boolean;
  nodeType?: EnumNodeType;
  vestingPledge?: PledgeData;
};

export type TPendingDelegation = {
  block_height: number;
};

export type TDelegation = {
  owner: string;
  node_identity: string;
  amount: DecCoin;
  block_height: number;
  proxy: string; // proxy address used to delegate the funds on behalf of another address
  pending?: TPendingDelegation;
};

export type TBondNymNodeArgs = TNymNodeSignatureArgs & {
  msgSignature: string;
  fee?: Fee;
};

export type TNymNodeSignatureArgs = {
  nymnode: NymNode;
  costParams: NodeCostParams;
  pledge: DecCoin;
};

export type TBondGatewayArgs = {
  gateway: Gateway;
  pledge: DecCoin;
  msgSignature: string;
  fee?: Fee;
};

export type TBondMixNodeArgs = {
  mixnode: MixNode;
  costParams: NodeCostParams;
  pledge: DecCoin;
  msgSignature: string;
  fee?: Fee;
};

export type TBondMixnodeSignatureArgs = {
  mixnode: MixNode;
  costParams: NodeCostParams;
  pledge: DecCoin;
  tokenPool: 'balance' | 'locked';
};

export type TBondGatewaySignatureArgs = {
  gateway: Gateway;
  pledge: DecCoin;
  tokenPool: 'balance' | 'locked';
};

export type TUpdateBondArgs = {
  currentPledge: DecCoin;
  newPledge: DecCoin;
  fee?: Fee;
};

export type TSimulateUpdateBondArgs = Omit<TUpdateBondArgs, 'fee'>;

export type TNodeDescription = {
  name: string;
  description: string;
  link: string;
  location: string;
};

export type TNodeConfigUpdateArgs = {
  host: string;
  custom_http_port: number;
};

export type TDelegateArgs = {
  identity: string;
  amount: DecCoin;
};

export type Period = 'Before' | { In: number } | 'After';

export type TAccount = {
  name: string;
  address: string;
  mnemonic: string;
};

export type TGatewayReport = {
  identity: string;
  owner: string;
  last_day: number;
  last_hour: number;
  most_recent: number;
};

export type TNodeRole = 'entryGateway' | 'exitGateway' | 'layer1' | 'layer2' | 'layer3' | 'standby';

export type MixnodeSaturationResponse = {
  saturation: string;
  uncapped_saturation: string;
};

export const isMixnode = (node: TBondedNode): node is TBondedMixnode => (node as TBondedMixnode).mixId !== undefined;

export const isGateway = (node: TBondedNode): node is TBondedGateway => (node as TBondedGateway).location !== undefined;

export const isNymNode = (node: TBondedNode): node is TBondedNymNode => !isMixnode(node) && !isGateway(node);
