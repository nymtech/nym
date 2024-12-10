// master APIs
export const API_BASE_URL = process.env.NEXT_PUBLIC_EXPLORER_API_URL || 'https://explorer.nymtech.net/api/v1';
export const NYM_API_BASE_URL = process.env.NEXT_PUBLIC_NYM_API_URL || 'https://validator.nymtech.net';

export const NYX_RPC_BASE_URL = process.env.NEXT_PUBLIC_NYX_RPC_BASE_URL || 'https://rpc.nymtech.net';

export const VALIDATOR_BASE_URL = process.env.NEXT_PUBLIC_VALIDATOR_URL || 'https://rpc.nymtech.net';
export const BLOCK_EXPLORER_BASE_URL = process.env.NEXT_PUBLIC_BIG_DIPPER_URL || 'https://nym.explorers.guru';

// specific API routes
export const OVERVIEW_API = `${API_BASE_URL}/overview`;
export const MIXNODE_PING = `${API_BASE_URL}/ping`;
export const MIXNODES_API = `${API_BASE_URL}/mix-nodes`;
export const MIXNODE_API = `${API_BASE_URL}/mix-node`;
export const VALIDATORS_API = `${NYX_RPC_BASE_URL}/validators`;
export const BLOCK_API = `${NYX_RPC_BASE_URL}/block`;
export const COUNTRY_DATA_API = `${API_BASE_URL}/countries`;
export const UPTIME_STORY_API = `${NYM_API_BASE_URL}/api/v1/status/mixnode`; // add ID then '/history' to this.
export const UPTIME_STORY_API_GATEWAY = `${NYM_API_BASE_URL}/api/v1/status/gateway`; // add ID then '/history' or '/report' to this
export const SERVICE_PROVIDERS = `${API_BASE_URL}/service-providers`;
export const TEMP_UNSTABLE_NYM_NODES = `${API_BASE_URL}/tmp/unstable/nym-nodes`;
export const TEMP_UNSTABLE_ACCOUNT = `${API_BASE_URL}/tmp/unstable/account`;
export const NYM_API_NODE_UPTIME = `${NYM_API_BASE_URL}/api/v1/nym-nodes/uptime-history`;
export const NYM_API_NODE_PERFORMANCE = `${NYM_API_BASE_URL}/api/v1/nym-nodes/performance-history`;

export const LEGACY_MIXNODES_API = `${API_BASE_URL}/tmp/unstable/legacy-mixnode-bonds`;
export const LEGACY_GATEWAYS_API = `${API_BASE_URL}/tmp/unstable/legacy-gateway-bonds`;

// errors
export const MIXNODE_API_ERROR = "We're having trouble finding that record, please try again or Contact Us.";

export const NYM_WEBSITE = 'https://nymtech.net';

export const EXPLORER_FOR_ACCOUNTS = ''; // set to empty to use this Nym Explorer and NOT an external one

export const NYM_MIXNET_CONTRACT =
  process.env.NYM_MIXNET_CONTRACT || 'n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr';
export const COSMOS_KIT_USE_CHAIN = process.env.NEXT_PUBLIC_COSMOS_KIT_USE_CHAIN || 'sandbox';
export const WALLET_CONNECT_PROJECT_ID = process.env.NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID || '';
