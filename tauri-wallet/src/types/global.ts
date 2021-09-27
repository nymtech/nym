import { Coin } from '.'

export enum EnumNodeType {
  mixnode = 'mixnode',
  gateway = 'gateway',
}

export type TNodeOwnership = {
  hasOwnership: boolean
  nodeType?: EnumNodeType
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

export type TFee = { [key in EnumNodeType]: Coin }
