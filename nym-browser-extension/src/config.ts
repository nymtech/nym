export const config = {
  rpcUrl: process.env.RPC_URL || '',
  validatorUrl: process.env.VALIDATOR_URL || '',
  prefix: process.env.PREFIX || '',
  mixnetContractAddress: process.env.MIXNET_CONTRACT_ADDRESS || '',
  vestingContractAddress: process.env.VESTING_CONTRACT_ADDRESS || '',
  denom: process.env.DENOM || '',
};
