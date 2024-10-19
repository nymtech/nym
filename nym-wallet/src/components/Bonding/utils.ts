import { Gateway, MixNode, NodeCostParams } from '@nymproject/types';
import { GatewayData, MixnodeAmount, MixnodeData } from '../../pages/bonding/types';
import { toPercentFloatString } from '../../utils';

export function mixnodeToTauri(data: MixnodeData): MixNode {
  return {
    mix_port: data.mixPort,
    http_api_port: data.httpApiPort,
    verloc_port: data.verlocPort,
    sphinx_key: data.sphinxKey,
    identity_key: data.identityKey,
    version: data.version,
    host: data.host,
  };
}

export function costParamsToTauri(data: MixnodeAmount): NodeCostParams {
  return {
    profit_margin_percent: toPercentFloatString(data.profitMargin),
    interval_operating_cost: {
      amount: data.operatorCost.amount.toString(),
      denom: data.operatorCost.denom,
    },
  };
}

export function gatewayToTauri(data: GatewayData): Gateway {
  return {
    host: data.host,
    version: data.version,
    mix_port: data.mixPort,
    clients_port: data.clientsPort,
    sphinx_key: data.sphinxKey,
    identity_key: data.identityKey,
    location: data.location,
  };
}
