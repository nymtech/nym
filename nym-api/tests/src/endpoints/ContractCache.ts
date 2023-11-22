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
    return response;
  }

  public async getMixnodesDetailed(): Promise<MixnodesDetailed[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/detailed`,
    });

    return response;
  }

  public async getGateways(): Promise<AllGateways[]> {
    const response = await this.restClient.sendGet({
      route: `gateways`,
    });
    return response;
  }

  public async getActiveMixnodes(): Promise<AllMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/active`,
    });
    return response;
  }

  public async getActiveMixnodesDetailed(): Promise<MixnodesDetailed[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/active/detailed`,
    });
    return response;
  }

  public async getRewardedMixnodes(): Promise<AllMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/rewarded`,
    });
    return response;
  }

  public async getRewardedMixnodesDetailed(): Promise<MixnodesDetailed[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/rewarded/detailed`,
    });
    return response;
  }

  public async getBlacklistedMixnodes(): Promise<[]> {
    const response = await this.restClient.sendGet({
      route: `mixnodes/blacklisted`,
    });
    return response;
  }

  public async getBlacklistedGateways(): Promise<[]> {
    const response = await this.restClient.sendGet({
      route: `gateways/blacklisted`,
    });
    return response;
  }

  public async getEpochRewardParams(): Promise<EpochRewardParams> {
    const response = await this.restClient.sendGet({
      route: `epoch/reward_params`,
    });
    return response;
  }

  public async getCurrentEpoch(): Promise<CurrentEpoch> {
    const response = await this.restClient.sendGet({
      route: `epoch/current`,
    });
    return response;
  }

  public async getServiceProviders(): Promise<ServiceProviders> {
    const response = await this.restClient.sendGet({
      route: `services`,
    });
    return response;
  }

  public async getNymAddressNames(): Promise<NymAddressNames> {
    const response = await this.restClient.sendGet({
      route: `names`,
    });
    return response;
  }
}
