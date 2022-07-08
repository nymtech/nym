import { MajorCurrencyAmount, TNodeType, TransactionExecuteResult } from '@nymproject/types';

export type FormStep = 1 | 2 | 3 | 4;
export type NodeType = TNodeType;
export type BondStatus = 'init' | 'success' | 'error' | 'loading';

export type ACTIONTYPE =
  | { type: 'change_bond_type'; payload: NodeType }
  | { type: 'set_node_data'; payload: NodeData }
  | { type: 'set_amount_data'; payload: AmountData }
  | { type: 'set_step'; payload: FormStep }
  | { type: 'set_tx'; payload: TransactionExecuteResult | undefined }
  | { type: 'set_bond_status'; payload: BondStatus }
  | { type: 'next_step' }
  | { type: 'prev_step' }
  | { type: 'show_modal' }
  | { type: 'close_modal' }
  | { type: 'reset' };

export type NodeIdentity = {
  identityKey: string;
  sphinxKey: string;
  signature: string;
  host: string;
  version: string;
  advancedOpt: boolean;
  mixPort: number;
};

export type MixnodeData = NodeIdentity & {
  verlocPort: number;
  httpApiPort: number;
};

export type MixnodeAmount = {
  amount: MajorCurrencyAmount;
  tokenPool: string;
  profitMargin: number;
};

export type GatewayData = NodeIdentity & {
  location: string;
  clientsPort: number;
};

export type GatewayAmount = Omit<MixnodeAmount, 'profitMargin'>;

export type NodeData<N = MixnodeData | GatewayData> = {
  nodeType: TNodeType;
} & N;

export interface AmountData {
  amount: MajorCurrencyAmount;
  tokenPool: string;
  profitMargin?: number;
}

export interface BondState {
  showModal: boolean;
  formStep: FormStep;
  nodeData?: NodeData;
  amountData?: MixnodeAmount | GatewayAmount;
  tx?: TransactionExecuteResult;
  bondStatus: BondStatus;
}
