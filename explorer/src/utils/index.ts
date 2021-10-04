import { MixNodeResponse, MixNodeResponseItem } from "src/typeDefs/explorer-api";


export function formatNumber(num: number) {
  return new Intl.NumberFormat().format(num);
}

export function scrollToRef(ref: any) {
  return ref.current.scrollIntoView();
}

export function mixnodeToGridRow(arrayOfMixnodes: MixNodeResponse) {
  let arr: any = [];
  arrayOfMixnodes !== undefined && arrayOfMixnodes.forEach((eachRecord: MixNodeResponseItem) => {
    let formattedRow = {
      id: eachRecord.owner,
      owner: eachRecord.owner,
      location: eachRecord?.location?.country_name || '',
      identity_key: eachRecord.mix_node.identity_key || '',
      bond: eachRecord.bond_amount.amount || '',
      host: eachRecord.mix_node.host || '',
      layer: eachRecord.layer || '',
    }
    arr.push(formattedRow);
  })
  return arr;
}