import { calculateStake, Console, decCoinToDisplay, fireRequests, TauriReq, toPercentIntegerString } from 'src/utils';
import { getNymNodeBondDetails } from './bond';
import { getNymNodeDescription, getNymNodePerformance, getPendingOperatorRewards } from './queries';
import { DecCoin } from '@nymproject/types';

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

    const { name, operatorRewards, uptime } = await getAdditionalNymNodeDetails(
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
    };
  } catch (e: any) {
    Console.warn(e);
    throw new Error(`While fetching current bond state, an error occurred: ${e}`);
  }
}

async function getAdditionalNymNodeDetails(_: number, host: string, port: number | null, clientAddress: string) {
  const details: {
    name: string;
    uptime: number;
    operatorRewards?: DecCoin;
  } = {
    name: 'Name has not been set',
    uptime: 0,
  };

  if (port) {
    const nodeDescription = await getNymNodeDescription(host, port);
    details.name = nodeDescription.name;
  }

  const uptimeReq: TauriReq<typeof getNymNodePerformance> = {
    name: 'getMixnodeAvgUptime',
    request: () => getNymNodePerformance(),
    onFulfilled: (value) => {
      details.uptime = value;
    },
  };

  const operatorRewardsReq: TauriReq<typeof getPendingOperatorRewards> = {
    name: 'getPendingOperatorRewards',
    request: () => getPendingOperatorRewards(clientAddress),
    onFulfilled: (value) => {
      details.operatorRewards = decCoinToDisplay(value);
    },
  };

  await fireRequests([operatorRewardsReq, uptimeReq]);

  return details;
}

type TBondedNymNodeResponse = Awaited<ReturnType<typeof getNymNodeDetails>>;
type TBondedNymNode = NonNullable<TBondedNymNodeResponse>;

export { getNymNodeDetails };
export type { TBondedNymNodeResponse, TBondedNymNode };
