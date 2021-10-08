import { GatewayResponse, GatewayResponseItem, MixNodeResponse, MixNodeResponseItem } from "src/typeDefs/explorer-api";


export function formatNumber(num: number) {
  return new Intl.NumberFormat().format(num);
}

export function scrollToRef(ref: any) {
  return ref.current.scrollIntoView();
}

type MixnodeRowType = {
  id: string
  owner: string
  location: string
  identity_key: string
  bond: number
  host: string
  layer: string
}
type GatewayRowType = {
  id: string
  owner: string
  identity_key: string
  bond: number
  host: string
  location: string
}

type GatewayRows = GatewayRowType[];
type MixnodeRows = MixnodeRowType[];

export function mixnodeToGridRow(arrayOfMixnodes: MixNodeResponse) {
  let arr: MixnodeRows = [];
  arrayOfMixnodes !== undefined && arrayOfMixnodes.forEach((eachRecord: MixNodeResponseItem) => {
    let formattedRow: MixnodeRowType = {
      id: eachRecord.owner,
      owner: eachRecord.owner,
      location: eachRecord?.location?.country_name || '',
      identity_key: eachRecord.mix_node.identity_key || '',
      bond: eachRecord.bond_amount.amount || 0,
      host: eachRecord.mix_node.host || '',
      layer: eachRecord.layer || '',
    }
    arr.push(formattedRow);
  })
  return arr;
}

export function gatewayToGridRow(arrayOfGateways: GatewayResponse) {
  let arr: GatewayRows = [];
  arrayOfGateways !== undefined && arrayOfGateways.forEach((eachRecord: GatewayResponseItem) => {
    let formattedRow: GatewayRowType = {
      id: eachRecord.owner,
      owner: eachRecord.owner,
      identity_key: eachRecord.gateway.identity_key || eachRecord.gateway.identity_key || '',
      location: eachRecord?.gateway?.location || '',
      bond: eachRecord.bond_amount.amount || 0,
      host: eachRecord.gateway.host || ''
    }
    arr.push(formattedRow);
  })
  return arr;
}