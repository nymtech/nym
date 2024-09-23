import { TNodeDescription } from 'src/types';
import { TauriReq, decCoinToDisplay, fireRequests } from 'src/utils';
import { getGatewayReport, getMixNodeDescription as getNodeDescriptionRequest } from './queries';
import { getGatewayBondDetails } from './bond';

async function getAdditionalGatewayDetails(identityKey: string, host: string, port: number) {
  const details: {
    routingScore?: { current: number; average: number } | undefined;
    nodeDescription?: TNodeDescription | undefined;
  } = {};

  const reportReq: TauriReq<typeof getGatewayReport> = {
    name: 'getGatewayReport',
    request: () => getGatewayReport(identityKey),
    onFulfilled: (value) => {
      details.routingScore = { current: value.most_recent, average: value.last_day };
    },
  };

  const nodeDescReq: TauriReq<typeof getNodeDescriptionRequest> = {
    name: 'getNodeDescription',
    request: () => getNodeDescriptionRequest(host, port),
    onFulfilled: (value) => {
      details.nodeDescription = value;
    },
  };

  await fireRequests([reportReq, nodeDescReq]);

  return details;
}

async function getGatewayDetails() {
  try {
    const data = await getGatewayBondDetails();
    if (!data) {
      return null;
    }

    const { gateway, proxy } = data;

    const { nodeDescription, routingScore } = await getAdditionalGatewayDetails(
      gateway.identity_key,
      gateway.host,
      gateway.clients_port,
    );

    return {
      name: nodeDescription?.name,
      identityKey: gateway.identity_key,
      mixPort: gateway.mix_port,
      httpApiPort: gateway.clients_port,
      host: gateway.host,
      ip: gateway.host,
      location: gateway.location,
      bond: decCoinToDisplay(data.pledge_amount),
      proxy,
      routingScore,
      version: gateway.version,
    };
  } catch (error) {
    console.error(error);
    return null;
  }
}

type TBondedGatewayResponse = Awaited<ReturnType<typeof getGatewayDetails>>;
type TBondedGateway = NonNullable<TBondedGatewayResponse>;

export { getGatewayDetails };
export type { TBondedGatewayResponse, TBondedGateway };
