export enum EnumNodeType {
  Mixnode = 'Mixnode',
  Gateway = 'Gateway',
}

export enum EnumDemon {
  upunk = 'upunk',
  punk = 'punk',
}

export type TNodeOwnership = {
  ownsMixnode: boolean
  ownsGateway: boolean
}

export type TBalance = {
  amount: string
  demon: EnumDemon
}

export type TClientDetails = {
  client_address: string
  contract_address: string
  denom: EnumDemon
}
