import {
  ActiveStatus,
  AvgUptime,
  CoreCount,
  EstimatedReward,
  InclusionProbability,
  NodeHistory,
  Report,
  StakeSaturation,
} from "../types/StatusTypes";
import { APIClient } from "./abstracts/APIClient";

export default class Status extends APIClient {
  constructor() {
    super("/status");
  }

  public async getMixnodeStatusReport(identity_key: string): Promise<Report> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/report`,
    });

    return response.data;
  }

  public async getGatewayStatusReport(identity_key: string): Promise<Report> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/report`,
    });

    return response.data;
  }

  public async getGatewayHistory(identity_key: string): Promise<NodeHistory> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/history`,
    });

    return response.data;
  }

  public async getMixnodeHistory(identity_key: string): Promise<NodeHistory> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/history`,
    });

    return response.data;
  }

  public async getMixnodeStakeSaturation(
    identity_key: string
  ): Promise<StakeSaturation> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/stake-saturation`,
    });

    return response.data;
  }

  public async getMixnodeCoreCount(identity_key: string): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/core-status-count`,
    });

    return response.data;
  }

  public async getGatewayCoreCount(identity_key: string): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/core-status-count`,
    });

    return response.data;
  }

  public async getMixnodeRewardComputation(
    identity_key: string
  ): Promise<EstimatedReward> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/reward-estimation`,
    });

    return response.data;
  }

  public async getMixnodeRewardEstimatedComputation(
    identity_key: string
  ): Promise<EstimatedReward> {
    const response = await this.restClient.sendPost({
      route: `/mixnode/${identity_key}/compute-reward-estimation`,
    });

    return response.data;
  }

  public async getMixnodeAverageUptime(
    identity_key: string
  ): Promise<AvgUptime> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/avg_uptime`,
    });

    return response.data;
  }

  public async getMixnodeInclusionProbability(
    identity_key: string
  ): Promise<InclusionProbability> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/inclusion-probability`,
    });

    return response.data;
  }

  public async getMixnodeStatus(identity_key: string): Promise<ActiveStatus> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/status`,
    });

    return response.data;
  }
}
