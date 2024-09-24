import { calculateStake, Console, decCoinToDisplay, fireRequests, TauriReq, toPercentIntegerString } from 'src/utils';
import { getNymNodeBondDetails } from './bond';
import {
  getNymNodeDescription,
  getNymNodeRole,
  getNymNodeStakeSaturation,
  getNymNodeUptime,
  getPendingOperatorRewards,
} from './queries';
import { DecCoin, decimalToFloatApproximation, decimalToPercentage } from '@nymproject/types';
import { TNodeRole } from 'src/types';

async function getNymNodeDetails(clientAddress: string) {
  try {
    const data = await getNymNodeBondDetails();

    if (!data) {
      return null;
    }

    const {
      bond_information,
      rewarding_details,
      bond_information: { node_id },
    } = data;

    console.log('bond_information', data);

    const { name, operatorRewards, uptime, stakeSaturation, uncappedSaturation, role } =
      await getAdditionalNymNodeDetails(
        node_id,
        bond_information.host,
        bond_information.custom_http_port,
        clientAddress,
      );

    return {
      name,
      nodeId: node_id,
      identityKey: bond_information.identity_key,
      stake: {
        amount: calculateStake(rewarding_details.operator, rewarding_details.delegates) || '0',
        denom: bond_information.original_pledge.denom,
      },
      bond: decCoinToDisplay(bond_information.original_pledge),
      profitMargin: toPercentIntegerString(rewarding_details.cost_params.profit_margin_percent),
      delegators: rewarding_details.unique_delegations,
      operatorCost: decCoinToDisplay(rewarding_details.cost_params.interval_operating_cost),
      host: bond_information.host.replace(/\s/g, ''),
      httpApiPort: bond_information.custom_http_port,
      isUnbonding: bond_information.is_unbonding,
      operatorRewards,
      uptime,
      stakeSaturation,
      uncappedStakeSaturation: uncappedSaturation,
      role,
    };
  } catch (e: any) {
    Console.warn(e);
    throw new Error(`While fetching current bond state, an error occurred: ${e}`);
  }
}

async function getAdditionalNymNodeDetails(nodeId: number, host: string, port: number | null, clientAddress: string) {
  const details: {
    name: string;
    uptime: number;
    operatorRewards?: DecCoin;
    stakeSaturation: string;
    uncappedSaturation?: number;
    role?: TNodeRole;
  } = {
    name: 'Name has not been set',
    uptime: 0,
    stakeSaturation: '0',
  };

  if (port) {
    const nodeDescription = await getNymNodeDescription(host, port);
    details.name = nodeDescription.name;
  }

  const uptimeReq: TauriReq<typeof getNymNodeUptime> = {
    name: 'getMixnodeAvgUptime',
    request: () => getNymNodeUptime(nodeId),
    onFulfilled: (value) => {
      details.uptime = value;
    },
  };

  const stakeSaturationReq: TauriReq<typeof getNymNodeStakeSaturation> = {
    name: 'getMixnodeStakeSaturation',
    request: () => getNymNodeStakeSaturation(nodeId),
    onFulfilled: (value) => {
      details.stakeSaturation = decimalToPercentage(value.current_saturation);
      const rawUncappedSaturation = decimalToFloatApproximation(value.uncapped_saturation);
      if (rawUncappedSaturation && rawUncappedSaturation > 1) {
        details.uncappedSaturation = Math.round(rawUncappedSaturation * 100);
      }
    },
  };

  const operatorRewardsReq: TauriReq<typeof getPendingOperatorRewards> = {
    name: 'getPendingOperatorRewards',
    request: () => getPendingOperatorRewards(clientAddress),
    onFulfilled: (value) => {
      details.operatorRewards = decCoinToDisplay(value);
    },
  };

  const getNymNodeRoleReq: TauriReq<typeof getNymNodeRole> = {
    name: 'getNymNodeRole',
    request: () => getNymNodeRole(nodeId),
    onFulfilled: (value) => {
      details.role = value;
    },
  };

  await fireRequests([operatorRewardsReq, uptimeReq, stakeSaturationReq, getNymNodeRoleReq]);

  return details;
}

type TBondedNymNodeResponse = Awaited<ReturnType<typeof getNymNodeDetails>>;
type TBondedNymNode = NonNullable<TBondedNymNodeResponse>;

export { getNymNodeDetails };
export type { TBondedNymNodeResponse, TBondedNymNode };
