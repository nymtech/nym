import { MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';

export type FormStep = 1 | 2 | 3 | 4;
export type NodeType = 'mixnode' | 'gateway';

export type ACTIONTYPE =
  | { type: 'change_bond_type'; payload: NodeType }
  | { type: 'set_node_data'; payload: NodeData }
  | { type: 'set_amount_data'; payload: AmountData }
  | { type: 'set_step'; payload: FormStep }
  | { type: 'set_tx'; payload: TransactionExecuteResult }
  | { type: 'next_step' }
  | { type: 'previous_step' }
  | { type: 'show_modal' }
  | { type: 'close_modal' }
  | { type: 'reset' };

export interface NodeData {
  nodeType: NodeType;
  identityKey: string;
  sphinxKey: string;
  signature: string;
  host: string;
  location?: string;
  version: string;
  advancedOpt: boolean;
  mixPort: number;
  verlocPort: number;
  clientsPort: number;
  httpApiPort: number;
}

export interface AmountData {
  amount: MajorCurrencyAmount;
  tokenPool: string;
  profitMargin?: number;
}

export interface BondState {
  showModal: boolean;
  formStep: FormStep;
  nodeData?: NodeData;
  amountData?: AmountData;
  tx?: TransactionExecuteResult;
}
