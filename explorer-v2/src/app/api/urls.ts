export const CURRENT_EPOCH =
  "https://validator.nymtech.net/api/v1/epoch/current";
export const CURRENT_EPOCH_REWARDS =
  "https://validator.nymtech.net/api/v1/epoch/reward_params";

export const NYM_ACCOUNT_ADDRESS =
  "https://explorer.nymtech.net/api/v1/tmp/unstable/account";
export const NYM_PRICES_API = "https://api.nym.spectredao.net/api/v1/nym-price";
export const VALIDATOR_BASE_URL =
  process.env.NEXT_PUBLIC_VALIDATOR_URL || "https://rpc.nymtech.net";
export const DATA_OBSERVATORY_NODES_URL =
  "https://api.nym.spectredao.net/api/v1/nodes";

export const DATA_OBSERVATORY_DELEGATIONS_URL =
  "https://api.nym.spectredao.net/api/v1/delegations";
export const DATA_OBSERVATORY_BALANCES_URL =
  "https://api.nym.spectredao.net/api/v1/balances";
export const OBSERVATORY_GATEWAYS_URL =
  "https://mainnet-node-status-api.nymtech.cc/v2/gateways";

export const NS_API_MIXNODES_STATS =
  process.env.NEXT_PUBLIC_NS_API_MIXNODES_STATS;

export const NS_API_NODES = process.env.NEXT_PUBLIC_NS_API_NODES;
