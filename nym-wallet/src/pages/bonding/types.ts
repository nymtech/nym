import { DecCoin, MixNodeCostParams, TNodeType, TransactionExecuteResult } from '@nymproject/types';
import { TPoolOption } from 'src/components';

export type FormStep = 1 | 2 | 3 | 4;
export type BondStatus = 'init' | 'success' | 'error' | 'loading';

export type NodeIdentity = {
  identityKey: string;
  sphinxKey: string;
  ownerSignature: string;
  host: string;
  version: string;
  mixPort: number;
};

export type MixnodeData = NodeIdentity & {
  verlocPort: number;
  httpApiPort: number;
};

export type Amount = {
  amount: DecCoin;
  operatorCost: DecCoin;
  tokenPool: string;
};

export type GatewayAmount = Amount;

export type MixnodeAmount = Amount & {
  profitMargin: string;
};

export type Signature = {
  signature: string;
};

export type GatewayData = NodeIdentity & {
  location: string;
  clientsPort: number;
};

export type NodeData<N = MixnodeData | GatewayData> = {
  nodeType: TNodeType;
} & N;

export interface AmountData {
  amount: DecCoin;
  tokenPool: TPoolOption;
  profitMargin?: number;
}

export interface BondState {
  showModal: boolean;
  formStep: FormStep;
  nodeData?: NodeData;
  amountData?: MixnodeAmount | GatewayAmount;
  tx?: TransactionExecuteResult;
  bondStatus: BondStatus;
  error?: string | null;
}

export interface ChangeMixCostParams {
  mix_id: number;
  new_costs: MixNodeCostParams;
}
