import type { DecCoin } from "../../../../ts-packages/types/src/types/rust/DecCoin";

export interface TauriContractStateParams { minimum_mixnode_pledge: DecCoin, minimum_gateway_pledge: DecCoin, minimum_mixnode_delegation: DecCoin | null, }