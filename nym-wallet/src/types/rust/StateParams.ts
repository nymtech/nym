import type { DecCoin } from '@nymproject/types';

export interface TauriContractStateParams {
  minimum_mixnode_pledge: DecCoin;
  minimum_gateway_pledge: DecCoin;
  minimum_mixnode_delegation: DecCoin | null;
}
