export enum EnumNodeType {
  Mixnode = 'Mixnode',
  Gateway = 'Gateway',
}

export type TNodeOwnership = {
  ownsMixnode: boolean
  ownsGateway: boolean
}

export type TClientDetails = {
  client_address: string
  contract_address: string
}
