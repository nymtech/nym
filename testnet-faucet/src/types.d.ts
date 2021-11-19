declare module '*.svg' {
  const content: any
  export default content
}

namespace NodeJS {
  export interface Process {
    env: {
      VALIDATOR_ADDRESS: string
      MNEMONIC: string
      TESTNET_URL_1: string
      ACCOUNT_ADDRESS: string
    }
  }
}
