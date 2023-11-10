import {
  ActiveStatus,
  AvgUptime,
  CoreCount,
  DetailedGateway,
  DetailedMixnodes,
  InclusionProbabilities,
  InclusionProbability,
  NodeHistory,
  ErrorMsg,
  Report,
  RewardEstimation,
  StakeSaturation,
} from "../types/StatusInterfaces";
import { APIClient } from "./abstracts/APIClient";

export default class Status extends APIClient {
  constructor() {
    super("/status");
  }

  // GATEWAYS

  public async getDetailedGateways(): Promise<DetailedGateway[]> {
    const response = await this.restClient.sendGet({
      route: `/gateways/detailed`,
    });

    return response;
  }

  public async getUnfilteredGateways(): Promise<DetailedGateway[]> {
    const response = await this.restClient.sendGet({
      route: `/gateways/detailed-unfiltered`,
    });

    return response;
  }

  public async getGatewayStatusReport(
    identity_key: string
  ): Promise<Report | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/report`,
    });

    return response;
  }

  public async getGatewayHistory(
    identity_key: string
  ): Promise<NodeHistory | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/history`,
    });

    return response;
  }

  public async getGatewayCoreCount(identity_key: string): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/core-status-count`,
    });

    return response;
  }

  public async getGatewayAverageUptime(
    identity_key: string
  ): Promise<CoreCount | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/avg_uptime`,
    });

    return response;
  }

  // MIXNODES

  public async getMixnodeStatusReport(
    mix_id: number
  ): Promise<Report | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/report`,
    });

    return response;
  }

  public async getMixnodeStakeSaturation(
    mix_id: number
  ): Promise<StakeSaturation | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/stake-saturation`,
    });

    return response;
  }

  public async getMixnodeCoreCount(mix_id: number): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/core-status-count`,
    });

    return response;
  }

  public async getMixnodeRewardComputation(
    mix_id: number
  ): Promise<RewardEstimation | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/reward-estimation`,
    });

    return response;
  }

  public async sendMixnodeRewardEstimatedComputation(
    mix_id: number
  ): Promise<RewardEstimation> {
    const response = await this.restClient.sendPost({
      route: `/mixnode/${mix_id}/compute-reward-estimation`,
      data: {
        // performance: "10",
        active_in_rewarded_set: true,
        // pledge_amount: 10,
        // total_delegation: 2000,
        // interval_operating_cost: {
        //   denom: "unym",
        //   amount: "250000000"
        // },
        // profit_margin_percent: 10
      },
    });

    return response;
  }

  public async getMixnodeHistory(
    mix_id: number
  ): Promise<NodeHistory | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/history`,
    });

    return response;
  }

  public async getMixnodeAverageUptime(
    mix_id: number
  ): Promise<AvgUptime | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/avg_uptime`,
    });

    return response;
  }

  public async getMixnodeInclusionProbability(
    mix_id: number
  ): Promise<InclusionProbability | ErrorMsg> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/inclusion-probability`,
    });

    return response;
  }

  public async getMixnodeStatus(mix_id: number): Promise<ActiveStatus> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/status`,
    });

    return response;
  }

  public async getAllMixnodeInclusionProbability(): Promise<InclusionProbabilities> {
    const response = await this.restClient.sendGet({
      route: `/mixnodes/inclusion_probability`,
    });

    return response;
  }

  public async getDetailedMixnodes(): Promise<DetailedMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `/mixnodes/detailed`,
    });

    return response;
  }

  public async getDetailedRewardedMixnodes(): Promise<DetailedMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `/mixnodes/rewarded/detailed`,
    });

    return response;
  }

  public async getUnfilteredMixnodes(): Promise<DetailedMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `/mixnodes/detailed-unfiltered`,
    });

    return response;
  }

  public async getDetailedActiveMixnodes(): Promise<DetailedMixnodes[]> {
    const response = await this.restClient.sendGet({
      route: `/mixnodes/active/detailed`,
    });

    return response;
  }
}
