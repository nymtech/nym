export const CURRENT_EPOCH =
  "https://validator.nymtech.net/api/v1/epoch/current";
export const SANDBOX_CURRENT_EPOCH =
  "https://sandbox-nym-api1.nymtech.net/api/v1/epoch/current";

export const CURRENT_EPOCH_REWARDS =
  "https://validator.nymtech.net/api/v1/epoch/reward_params";
export const SANDBOX_CURRENT_EPOCH_REWARDS =
  "https://sandbox-nym-api1.nymtech.net/api/v1/epoch/reward_params";

export const NYM_ACCOUNT_ADDRESS =
  "https://validator.nymtech.net/api/v1/unstable/account";
export const SANDBOX_NYM_ACCOUNT_ADDRESS =
    "https://sandbox-nym-api1.nymtech.net/api/v1/unstable/account/";

export const NYM_PRICES_API = "https://api.nym.spectredao.net/api/v1/nym-price";

export const VALIDATOR_BASE_URL =
  process.env.NEXT_PUBLIC_VALIDATOR_URL || "https://rpc.nymtech.net";
export const SANDBOX_VALIDATOR_BASE_URL = "https://rpc.sandbox.nymtech.net";

export const DATA_OBSERVATORY_BALANCES_URL =
  "https://api.nym.spectredao.net/api/v1/balances";


export const OBSERVATORY_GATEWAYS_URL =
  "https://mainnet-node-status-api.nymtech.cc/v2/gateways";
export const SANDBOX_OBSERVATORY_GATEWAYS_URL =
  "https://sandbox-node-status-api.nymte.ch/v2/gateways";

export const NS_API_MIXNODES_STATS =
  process.env.NEXT_PUBLIC_NS_API_MIXNODES_STATS;
export const SANDBOX_NS_API_MIXNODES_STATS =
  "https://sandbox-node-status-api.nymte.ch/v2/mixnodes/stats";

export const NS_API_NODES = process.env.NEXT_PUBLIC_NS_API_NODES;
export const SANDBOX_NS_API_NODES =
  "https://sandbox-node-status-api.nymte.ch/explorer/v3/nym-nodes";



