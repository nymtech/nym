import {
  MixnodesDetailed,
  AllGateways,
  AllMixnodes,
  EpochRewardParams,
  CurrentEpoch,
  ServiceProviders,
  NymAddressNames,
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

  public async getActiveMixnodesDetailed(): Promise<MixnodesDetailed[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/active/detailed`,
    });
    return response.data;
  }

  public async getRewardedMixnodes(): Promise<AllMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/rewarded`,
    });
    return response.data;
  }

  public async getRewardedMixnodesDetailed(): Promise<MixnodesDetailed[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/rewarded/detailed`,
    });
    return response.data;
  }

  public async getBlacklistedMixnodes(): Promise<[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/blacklisted`,
    });
    return response.data;
  }

  public async getBlacklistedGateways(): Promise<[]> {
    const response = await this.restClient.sendGet({
      route: `gateways/blacklisted`,
    });
    return response.data;
  }

  public async getEpochRewardParams(): Promise<EpochRewardParams> {
    const response = await this.restClient.sendGet({
      route: `epoch/reward_params`,
    });
    return response.data;
  }

  public async getCurrentEpoch(): Promise<CurrentEpoch> {
    const response = await this.restClient.sendGet({
      route: `epoch/current`,
    });
    return response.data;
  }

  public async getServiceProviders(): Promise<ServiceProviders> {
    const response = await this.restClient.sendGet({
      route: `services`,
    });
    return response.data;
  }

  public async getNymAddressNames(): Promise<NymAddressNames> {
    const response = await this.restClient.sendGet({
      route: `names`,
    });
    return response.data;
  }
}
