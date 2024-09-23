import Big from 'big.js';
import {
  DecCoin,
  decimalToFloatApproximation,
  decimalToPercentage,
  InclusionProbabilityResponse,
  MixnodeStatus,
} from '@nymproject/types';
import { Console } from 'src/utils/console';
import { TNodeDescription } from 'src/types';
import { TauriReq, unymToNym, decCoinToDisplay, fireRequests, toPercentIntegerString, calculateStake } from 'src/utils';
import {
  getMixnodeStatus,
  getMixnodeUptime,
  getMixnodeStakeSaturation,
  getMixnodeRewardEstimation,
  getInclusionProbability,
  getMixnodeAvgUptime,
  getNodeDescription as getNodeDescriptionRequest,
  getPendingOperatorRewards,
} from './queries';
import { getMixnodeBondDetails } from './bond';

async function getAdditionalMixnodeDetails(mixId: number, host: string, port: number, client_address: string) {
  const details: {
    status: MixnodeStatus;
    stakeSaturation: string;
    estimatedRewards?: DecCoin;
    uptime: number;
    averageUptime?: number;
    setProbability?: InclusionProbabilityResponse;
    nodeDescription?: TNodeDescription | undefined;
    operatorRewards?: DecCoin;
    uncappedSaturation?: number;
  } = {
    status: 'not_found',
    stakeSaturation: '0',
    uptime: 0,
  };

  const statusReq: TauriReq<typeof getMixnodeStatus> = {
    name: 'getMixnodeStatus',
    request: () => getMixnodeStatus(mixId),
    onFulfilled: (value) => {
      details.status = value.status;
    },
  };

  const uptimeReq: TauriReq<typeof getMixnodeUptime> = {
    name: 'getMixnodeUptime',
    request: () => getMixnodeUptime(mixId),
    onFulfilled: (value) => {
      details.uptime = value;
    },
  };

  const stakeSaturationReq: TauriReq<typeof getMixnodeStakeSaturation> = {
    name: 'getMixnodeStakeSaturation',
    request: () => getMixnodeStakeSaturation(mixId),
    onFulfilled: (value) => {
      details.stakeSaturation = decimalToPercentage(value.saturation);
      const rawUncappedSaturation = decimalToFloatApproximation(value.uncapped_saturation);
      if (rawUncappedSaturation && rawUncappedSaturation > 1) {
        details.uncappedSaturation = Math.round(rawUncappedSaturation * 100);
      }
    },
  };

  const rewardReq: TauriReq<typeof getMixnodeRewardEstimation> = {
    name: 'getMixnodeRewardEstimation',
    request: () => getMixnodeRewardEstimation(mixId),
    onFulfilled: (value) => {
      const estimatedRewards = unymToNym(value.estimation.total_node_reward);
      if (estimatedRewards) {
        details.estimatedRewards = {
          amount: estimatedRewards,
          denom: 'nym',
        };
      }
    },
  };

  const inclusionReq: TauriReq<typeof getInclusionProbability> = {
    name: 'getInclusionProbability',
    request: () => getInclusionProbability(mixId),
    onFulfilled: (value) => {
      details.setProbability = value;
    },
  };

  const avgUptimeReq: TauriReq<typeof getMixnodeAvgUptime> = {
    name: 'getMixnodeAvgUptime',
    request: () => getMixnodeAvgUptime(),
    onFulfilled: (value) => {
      details.averageUptime = value as number | undefined;
    },
  };

  const nodeDescReq: TauriReq<typeof getNodeDescriptionRequest> = {
    name: 'getNodeDescription',
    request: () => getNodeDescriptionRequest(host, port),
    onFulfilled: (value) => {
      details.nodeDescription = value;
    },
  };

  const operatorRewardsReq: TauriReq<typeof getPendingOperatorRewards> = {
    name: 'getPendingOperatorRewards',
    request: () => getPendingOperatorRewards(client_address),
    onFulfilled: (value) => {
      details.operatorRewards = decCoinToDisplay(value);
    },
  };

  await fireRequests([
    statusReq,
    uptimeReq,
    stakeSaturationReq,
    rewardReq,
    inclusionReq,
    avgUptimeReq,
    nodeDescReq,
    operatorRewardsReq,
  ]);

  return details;
}

async function getMixnodeDetails(client_address: string) {
  try {
    const data = await getMixnodeBondDetails();

    if (!data) {
      return null;
    }

    const {
      bond_information,
      rewarding_details,
      bond_information: { mix_id },
    } = data;

    const {
      status,
      stakeSaturation,
      uncappedSaturation: uncappedStakeSaturation,
      estimatedRewards,
      uptime,
      operatorRewards,
      averageUptime,
      nodeDescription,
      setProbability,
    } = await getAdditionalMixnodeDetails(
      mix_id,
      bond_information.mix_node.host,
      bond_information.mix_node.http_api_port,
      client_address,
    );

    return {
      name: nodeDescription?.name,
      mixId: mix_id,
      identityKey: bond_information.mix_node.identity_key,
      stake: {
        amount: calculateStake(rewarding_details.operator, rewarding_details.delegates) || '0',
        denom: bond_information.original_pledge.denom,
      },
      bond: decCoinToDisplay(bond_information.original_pledge),
      profitMargin: toPercentIntegerString(rewarding_details.cost_params.profit_margin_percent),
      delegators: rewarding_details.unique_delegations,
      proxy: bond_information.proxy,
      operatorRewards,
      uptime,
      status,
      stakeSaturation,
      uncappedStakeSaturation,
      operatorCost: decCoinToDisplay(rewarding_details.cost_params.interval_operating_cost),
      host: bond_information.mix_node.host.replace(/\s/g, ''),
      routingScore: averageUptime,
      activeSetProbability: setProbability?.in_active,
      standbySetProbability: setProbability?.in_reserve,
      estimatedRewards,
      httpApiPort: bond_information.mix_node.http_api_port,
      mixPort: bond_information.mix_node.mix_port,
      verlocPort: bond_information.mix_node.verloc_port,
      version: bond_information.mix_node.version,
      isUnbonding: bond_information.is_unbonding,
    };
  } catch (e: any) {
    Console.warn(e);
    throw new Error(`While fetching current bond state, an error occurred: ${e}`);
  }
}

type TBondedMixnodeResponse = Awaited<ReturnType<typeof getMixnodeDetails>>;
type TBondedMixnode = NonNullable<TBondedMixnodeResponse>;

export { getMixnodeDetails };
export type { TBondedMixnodeResponse, TBondedMixnode };
