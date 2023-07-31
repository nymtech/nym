import { Network } from './types';

const QA_VALIDATOR_URL = 'https://qa-nym-api.qa.nymte.ch/api';
const QWERTY_VALIDATOR_URL = 'https://qwerty-validator-api.qa.nymte.ch/api';
const SANDBOX_VALIDATOR_URL = 'https://sandbox-nym-api1.nymtech.net/api';
const MAINNET_VALIDATOR_URL = 'https://validator.nymtech.net/api';

const validatorApiFromNetwork = (network: Network) => {
  switch (network) {
    case 'QA':
      return QA_VALIDATOR_URL;
    case 'SANDBOX':
      return SANDBOX_VALIDATOR_URL;
    case 'MAINNET':
      return MAINNET_VALIDATOR_URL;
    default:
      throw new Error(`Unknown network: ${network}`);
  }
};

export {
  QA_VALIDATOR_URL,
  QWERTY_VALIDATOR_URL,
  MAINNET_VALIDATOR_URL,
  SANDBOX_VALIDATOR_URL,
  validatorApiFromNetwork,
};
