import {
  MixnodesDetailed,
  AllGateways,
  AllMixnodes,
  EpochRewardParams,
  BlacklistedGateways,
  BlacklistedMixnodes,
  CurrentEpoch
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

  public async getMixnodesDetailed(): Promise<MixnodesDetailed[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/detailed`,
    });

    return response.data;
  }

  public async getGateways(): Promise<AllGateways[]> {
    const response = await this.restClient.sendGet({
      route: `gateways`,
    });
    return response.data;
  }

  public async getActiveMixnodes(): Promise<AllMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/active`,
    });
    return response.data;
  }

  public async getBlacklistedMixnodes(): Promise<BlacklistedMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/blacklisted`,
    });
    return response.data;
  }

  public async getBlacklistedGateways(): Promise<BlacklistedGateways[]> {
    const response = await this.restClient.sendGet({
      route: `gateways/blacklisted`,
    });
    return response.data;
  }

  public async getEpochRewardParams(): Promise<EpochRewardParams> {
    const response = await this.restClient.sendGet({
      route: `epoch/reward_params`
    });
        console.log(response.data)
    return response.data;
  }

  public async getCurrentEpoch(): Promise<CurrentEpoch> {
    const response = await this.restClient.sendGet({
      route: `epoch/current`
    });
        console.log(response.data)
    return response.data;
  }

}
