import { GatewayResponse, GatewayBond, GatewayReportResponse } from '../typeDefs/explorer-api';
import { toPercentIntegerString } from '../utils';

export type GatewayRowType = {
  id: string;
  owner: string;
  identity_key: string;
  bond: number;
  host: string;
  location: string;
  version: string;
  node_performance: string;
};

export type GatewayEnrichedRowType = GatewayRowType & {
  routingScore: string;
  avgUptime: string;
  clientsPort: number;
  mixPort: number;
};

export function gatewayToGridRow(arrayOfGateways: GatewayResponse): GatewayRowType[] {
  return !arrayOfGateways
    ? []
    : arrayOfGateways.map((gw) => ({
        id: gw.owner,
        owner: gw.owner,
        identity_key: gw.gateway.identity_key || '',
        location: gw?.gateway?.location || '',
        bond: gw.pledge_amount.amount || 0,
        host: gw.gateway.host || '',
        version: gw.gateway.version || '',
        node_performance: toPercentIntegerString(gw.node_performance.last_24h),
      }));
}

export function gatewayEnrichedToGridRow(gateway: GatewayBond, report: GatewayReportResponse): GatewayEnrichedRowType {
  return {
    id: gateway.owner,
    owner: gateway.owner,
    identity_key: gateway.gateway.identity_key || '',
    location: gateway?.gateway?.location || '',
    bond: gateway.pledge_amount.amount || 0,
    host: gateway.gateway.host || '',
    version: gateway.gateway.version || '',
    clientsPort: gateway.gateway.clients_port || 0,
    mixPort: gateway.gateway.mix_port || 0,
    routingScore: `${report.most_recent}%`,
    avgUptime: `${report.last_day || report.last_hour}%`,
    node_performance: toPercentIntegerString(gateway.node_performance.most_recent),
  };
}
