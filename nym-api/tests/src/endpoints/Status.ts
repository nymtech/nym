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
  
  // GATEWAYS

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

  public async getGatewayCoreCount(identity_key: string): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/gateway/${identity_key}/core-status-count`,
    });

    return <CoreCount>{
      identity: response.data.identity,
      count: response.data.count,
    };
  }

  
  // MIXNODES

  public async getMixnodeStatusReport(mix_id: number): Promise<Report> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/report`,
    });

    return <Report>{
      mix_id: response.data.mix_id,
      identity: response.data.identity,
      owner: response.data.owner,
      most_recent: response.data.most_recent,
      last_hour: response.data.last_hour,
      last_day: response.data.last_day,
    };
  }

  public async getMixnodeStakeSaturation(
    mix_id: number
  ): Promise<StakeSaturation> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/stake-saturation`,
    });

    return <StakeSaturation>{
      as_at: response.data.as_at,
      saturation: response.data.saturation,
      uncapped_saturation: response.uncapped_saturation,
    };
  }

  public async getMixnodeCoreCount(mix_id: number): Promise<CoreCount> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/core-status-count`,
    });

    return <CoreCount>{
      mix_id: response.data.mix_id,
      identity: response.data.identity,
      count: response.data.count,
    };
  }

  public async getMixnodeRewardComputation(
    mix_id: number
  ): Promise<EstimatedReward> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/reward-estimation`,
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
    mix_id: number
  ): Promise<EstimatedReward> {
    const response = await this.restClient.sendPost({
      route: `/mixnode/${mix_id}/compute-reward-estimation`,
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

  public async getMixnodeHistory(mix_id: number): Promise<NodeHistory> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/history`,
    });

    return <NodeHistory>{
      mix_id: response.data.mix_id,
      identity: response.data.identity,
      owner: response.data.owner,
      history: response.data.history,
    };
  }

  public async getMixnodeAverageUptime(
    mix_id: number
  ): Promise<AvgUptime> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/avg_uptime`,
    });

    return <AvgUptime>{
      mix_id: response.data.mix_id,
      avg_uptime: response.data.avg_uptime,
    };
  }

  public async getMixnodeInclusionProbability(
    mix_id: number
  ): Promise<InclusionProbability> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/inclusion-probability`,
    });

    return <InclusionProbability>{
      in_active: response.data.in_active,
      in_reserve: response.data.in_reserve,
    };
  }

  public async getMixnodeStatus(mix_id: number): Promise<ActiveStatus> {
    const response = await this.restClient.sendGet({
      route: `/mixnode/${mix_id}/status`,
    });

    return <ActiveStatus>{
      status: response.data.status,
    };
  }
}
