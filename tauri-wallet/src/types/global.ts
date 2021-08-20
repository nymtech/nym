export enum EnumNodeType {
  Mixnode = 'Mixnode',
  Gateway = 'Gateway',
}

export type TNodeOwnership = {
  ownsMixnode: boolean
  ownsGateway: boolean
}
