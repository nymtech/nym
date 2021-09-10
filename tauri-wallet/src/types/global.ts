export enum EnumNodeType {
  mixnode = 'mixnode',
  gateway = 'gateway',
}

export type TNodeOwnership = {
  ownsMixnode: boolean
  ownsGateway: boolean
}

export type TClientDetails = {
  client_address: string
  contract_address: string
}

export type TSignInWithMnemonic = {
  denom: string
} & TClientDetails

export type TCreateAccount = {
  mnemonic: string
} & TSignInWithMnemonic
