import { GatewayResponse, GatewayResponseItem, GatewayReportResponse } from '../typeDefs/explorer-api';

export type GatewayRowType = {
  id: string;
  owner: string;
  identity_key: string;
  bond: number;
  host: string;
  location: string;
};

export type GatewayEnrichedRowType = GatewayRowType & {
  routing_score: string;
  avg_uptime: string;
  clients_port: number;
  mix_port: number;
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
      }));
}

export function gatewayEnrichedToGridRow(
  gateway: GatewayResponseItem,
  report: GatewayReportResponse,
): GatewayEnrichedRowType {
  return {
    id: gateway.owner,
    owner: gateway.owner,
    identity_key: gateway.gateway.identity_key || '',
    location: gateway?.gateway?.location || '',
    bond: gateway.pledge_amount.amount || 0,
    host: gateway.gateway.host || '',
    clients_port: gateway.gateway.clients_port || 0,
    mix_port: gateway.gateway.mix_port || 0,
    routing_score: `${report.most_recent}%`,
    avg_uptime: `${report.last_day || report.last_hour}%`,
  };
}
