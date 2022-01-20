// master APIs
export const API_BASE_URL = process.env.EXPLORER_API_URL;
export const VALIDATOR_API_BASE_URL = process.env.VALIDATOR_API_URL;
export const BIG_DIPPER = process.env.BIG_DIPPER_URL;

// specific API routes
export const OVERVIEW_API = `${API_BASE_URL}/overview`;
export const MIXNODE_PING = `${API_BASE_URL}/ping`;
export const MIXNODES_API = `${API_BASE_URL}/mix-nodes`;
export const MIXNODE_API = `${API_BASE_URL}/mix-node`;
export const GATEWAYS_API = `${VALIDATOR_API_BASE_URL}/api/v1/gateways`;
export const VALIDATORS_API = `${VALIDATOR_API_BASE_URL}/validators`;
export const BLOCK_API = `${VALIDATOR_API_BASE_URL}/block`;
export const COUNTRY_DATA_API = `${API_BASE_URL}/countries`;
export const UPTIME_STORY_API = `${VALIDATOR_API_BASE_URL}/api/v1/status/mixnode`; // add ID then '/history' to this.

// errors
export const MIXNODE_API_ERROR =
  "We're having trouble finding that record, please try again or Contact Us.";

// socials
export const TELEGRAM_LINK = 'https://t.me/nymchan';
export const TWITTER_LINK = 'https://twitter.com/nymproject';
export const GITHUB_LINK = 'https://github.com/nymtech';
export const NYM_WEBSITE = 'https://nymtech.net';
