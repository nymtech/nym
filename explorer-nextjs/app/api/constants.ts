// master APIs
export const API_BASE_URL = process.env.NEXT_PUBLIC_EXPLORER_API_URL;
export const NYM_API_BASE_URL = process.env.NEXT_PUBLIC_NYM_API_URL;
export const VALIDATOR_BASE_URL = process.env.NEXT_PUBLIC_VALIDATOR_URL || 'https://rpc.nymtech.net';
export const BIG_DIPPER = process.env.NEXT_PUBLIC_BIG_DIPPER_URL;

// specific API routes
export const OVERVIEW_API = `${API_BASE_URL}/overview`;
export const MIXNODE_PING = `${API_BASE_URL}/ping`;
export const MIXNODES_API = `${API_BASE_URL}/mix-nodes`;
export const MIXNODE_API = `${API_BASE_URL}/mix-node`;
export const GATEWAYS_EXPLORER_API = `${API_BASE_URL}/gateways`;
export const GATEWAYS_API = `${NYM_API_BASE_URL}/api/v1/status/gateways/detailed`;
export const VALIDATORS_API = `${VALIDATOR_BASE_URL}/validators`;
export const BLOCK_API = `${NYM_API_BASE_URL}/block`;
export const COUNTRY_DATA_API = `${API_BASE_URL}/countries`;
export const UPTIME_STORY_API = `${NYM_API_BASE_URL}/api/v1/status/mixnode`; // add ID then '/history' to this.
export const UPTIME_STORY_API_GATEWAY = `${NYM_API_BASE_URL}/api/v1/status/gateway`; // add ID then '/history' or '/report' to this
export const SERVICE_PROVIDERS = `${API_BASE_URL}/service-providers`;

// errors
export const MIXNODE_API_ERROR = "We're having trouble finding that record, please try again or Contact Us.";

export const NYM_WEBSITE = 'https://nymtech.net';

export const NYM_BIG_DIPPER = 'https://mixnet.explorers.guru';

export const NYM_MIXNET_CONTRACT =
  process.env.NYM_MIXNET_CONTRACT || 'n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr';
export const COSMOS_KIT_USE_CHAIN = process.env.NEXT_PUBLIC_COSMOS_KIT_USE_CHAIN || 'sandbox';
export const WALLET_CONNECT_PROJECT_ID = process.env.NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID || '';
