declare module '*.svg' {
  const content: any
  export default content
}

namespace NodeJS {
  export interface Process {
    env: {
      VALIDATOR_CONTRACT: string
      MNEMONIC: string
      TESTNET_URL: string
      ACCOUNT_ADDRESS: string
      MAJOR_CURRENCY: string
      MINOR_CURRENCY: string
      NETWORK: string
    }
  }
}
