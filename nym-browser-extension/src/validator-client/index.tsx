import ValidatorClient from '@nymproject/nym-validator-client';
import { config } from 'src/config';

export const generateMnemonmic = () => ValidatorClient.randomMnemonic();

export const connectToValidator = async (mnemonic: string) =>
  ValidatorClient.connect(
    mnemonic,
    config.rpcUrl,
    config.validatorUrl,
    config.prefix,
    config.mixnetContractAddress,
    config.vestingContractAddress,
    config.denom,
  );
