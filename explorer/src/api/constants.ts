// master APIs
export const MASTER_URL = 'https://testnet-milhon-explorer.nymtech.net';
export const MASTER_VALIDATOR_URL =
  'https://testnet-milhon-validator1.nymtech.net';
export const BIG_DIPPER = 'https://testnet-milhon-blocks.nymtech.net';

// specific API routes
export const MIXNODE_PING = `${MASTER_URL}/api/ping`;
export const MIXNODES_API = `${MASTER_URL}/api/mix-node`;
export const GATEWAYS_API = `${MASTER_VALIDATOR_URL}/api/v1/gateways`;
export const VALIDATORS_API = `${MASTER_VALIDATOR_URL}/validators`;
export const BLOCK_API = `${MASTER_VALIDATOR_URL}/block`;
export const COUNTRY_DATA_API = `${MASTER_URL}/api/countries`;
export const UPTIME_STORY_API = `${MASTER_VALIDATOR_URL}/api/v1/status/mixnode`; // add ID then '/history' to this.

// errors
export const MIXNODE_API_ERROR =
  "We're having trouble finding that record, please try again or Contact Us.";

// socials
export const TELEGRAM_LINK = 'https://t.me/nymchan';
export const TWITTER_LINK = 'https://twitter.com/nymproject';
export const GITHUB_LINK = 'https://github.com/nymtech';
