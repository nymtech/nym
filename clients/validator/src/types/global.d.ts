declare namespace NodeJS {
  interface ProcessEnv {
    rpcAddress: string;
    validatorAddress: string;
    prefix: string;
    mixnetContractAddress: string;
    vestingContractAddress: string;
    denom: string;
    mnemonic: string;
  }
}
