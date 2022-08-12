import {
  MixnodesDetailed,
  AllGateways,
  AllMixnodes,
} from "../types/ContractCacheTypes";
import { APIClient } from "./abstracts/APIClient";

export default class ContractCache extends APIClient {
  constructor() {
    super("/");
  }

  public async getMixnodes(): Promise<AllMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes`,
    });

    return response.data;
  }

  public async getMixnodesDetailed(): Promise<MixnodesDetailed> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/detailed`,
    });

    return <MixnodesDetailed>{
      mixnode_bond: response.data.mixnode_bond,
      stake_saturation: response.data.stake_saturation,
      uptime: response.data.uptime,
      estimated_operator_apy: response.data.estimated_operator_apy,
      estimated_delegators_apy: response.data.estimated_delegators_apy,
    };
  }

  public async getGateways(): Promise<AllGateways> {
    const response = await this.restClient.sendGet({
      route: `gateways`,
    });

    return <AllGateways>{
      pledge_amount: response.data.pledge_amount,
      owner: response.data.owner,
      block_height: response.data.block_height,
      gateway: response.data.gateway,
      proxy: response.data.proxy,
    };
  }
}
