import { AxiosResponse } from "axios";
import {
  ActiveStatus,
  AvgUptime,
  CoreCount,
  EstimatedReward,
  InclusionProbability,
  NodeHistory,
  Report,
  StakeSaturation,
} from "../../src/interfaces/StatusInterfaces";
import { APIClient } from "./abstracts/APIClient";

export default class Status extends APIClient {
  constructor() {
    super("/status");
  }

  public async getMixnodeStatusReport(identity_key: string): Promise<Report> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/report`,
    });

    return <Report>{
      identity: response.data.identity,
      owner: response.data.owner,
      most_recent: response.data.most_recent,
      last_hour: response.data.last_hour,
      last_day: response.data.last_day,
    };
  }

  public async getGatewayStatusReport(identity_key: string): Promise<Report> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/report`,
    });

    return <Report>{
      identity: response.data.identity,
      owner: response.data.owner,
      most_recent: response.data.most_recent,
      last_hour: response.data.last_hour,
      last_day: response.data.last_day,
    };
  }

  public async getGatewayHistory(identity_key: string): Promise<NodeHistory> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/history`,
    });

    return <NodeHistory>{
      identity: response.data.identity,
      owner: response.data.owner,
      history: response.data.history,
    };
  }

  public async getMixnodeStakeSaturation(
    identity_key: string
  ): Promise<StakeSaturation> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/stake-saturation`,
    });

    return <StakeSaturation>{
      as_at: response.data.as_at,
      saturation: response.data.saturation,
    };
  }

  public async getMixnodeCoreCount(identity_key: string): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/core-status-count`,
    });

    return <CoreCount>{
      identity: response.data.identity,
      count: response.data.count,
    };
  }

  public async getGatewayCoreCount(identity_key: string): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/core-status-count`,
    });

    return <CoreCount>{
      identity: response.data.identity,
      count: response.data.count,
    };
  }

  public async getMixnodeRewardComputation(
    identity_key: string
  ): Promise<EstimatedReward> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/reward-estimation`,
    });

    return <EstimatedReward>{
      estimated_total_node_reward: response.data.estimated_total_node_reward,
      estimated_operator_reward: response.data.estimated_operator_reward,
      estimated_delegators_reward: response.data.estimated_delegators_reward,
      estimated_node_profit: response.data.estimated_node_profit,
      estimated_operator_cost: response.data.estimated_operator_cost,
      reward_params: response.data.reward_params,
      as_at: response.data.as_at,
    };
  }

  public async getMixnodeRewardEstimatedComputation(
    identity_key: string
  ): Promise<EstimatedReward> {
    const response = await this.restClient.sendPost({
      route: `/mixnode/${identity_key}/compute-reward-estimation`,
    });

    return <EstimatedReward>{
      estimated_total_node_reward: response.data.estimated_total_node_reward,
      estimated_operator_reward: response.data.estimated_operator_reward,
      estimated_delegators_reward: response.data.estimated_delegators_reward,
      estimated_node_profit: response.data.estimated_node_profit,
      estimated_operator_cost: response.data.estimated_operator_cost,
      reward_params: response.data.reward_params,
      as_at: response.data.as_at,
    };
  }

  public async getMixnodeHistory(identity_key: string): Promise<NodeHistory> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/history`,
    });

    return <NodeHistory>{
      identity: response.data.identity,
      owner: response.data.owner,
      history: response.data.history,
    };
  }

  public async getMixnodeAverageUptime(
    identity_key: string
  ): Promise<AvgUptime> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/avg_uptime`,
    });

    return <AvgUptime>{
      identity: response.data.identity,
      avg_uptime: response.data.avg_uptime,
    };
  }

  public async getMixnodeInclusionProbability(
    identity_key: string
  ): Promise<InclusionProbability> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/inclusion-probability`,
    });

    return <InclusionProbability>{
      in_active: response.data.in_active,
      in_reserve: response.data.in_reserve,
    };
  }

  public async getMixnodeStatus(identity_key: string): Promise<ActiveStatus> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${identity_key}/status`,
    });

    return <ActiveStatus>{
      status: response.data.status,
    };
  }
}
