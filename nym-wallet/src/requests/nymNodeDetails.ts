import { calculateStake, Console, decCoinToDisplay, toPercentIntegerString } from 'src/utils';
import { getNymNodeBondDetails } from './bond';

async function getNymNodeDetails(client_address: string) {
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

    return {
      mixId: node_id,
      identityKey: bond_information.identity_key,
      stake: {
        amount: calculateStake(rewarding_details.operator, rewarding_details.delegates) || '0',
        denom: bond_information.original_pledge.denom,
      },
      bond: decCoinToDisplay(bond_information.original_pledge),
      profitMargin: toPercentIntegerString(rewarding_details.cost_params.profit_margin_percent),
      delegators: rewarding_details.unique_delegations,
      status,
      operatorCost: decCoinToDisplay(rewarding_details.cost_params.interval_operating_cost),
      host: bond_information.host.replace(/\s/g, ''),
      httpApiPort: bond_information.custom_http_port,
      isUnbonding: bond_information.is_unbonding,
    };
  } catch (e: any) {
    Console.warn(e);
    throw new Error(`While fetching current bond state, an error occurred: ${e}`);
  }
}

export { getNymNodeDetails };
