export interface TauriContractStateParams {
  minimum_mixnode_pledge: string; // TODO: handle string on Rust operation
  minimum_gateway_pledge: string;
  minimum_mixnode_delegation: string | null;
}
