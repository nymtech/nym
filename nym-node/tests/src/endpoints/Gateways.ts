import {
  ClientInterfaces,
  MixnetWebsockets,
  Wireguard,
} from "../types/GatewayTypes";
import { APIClient } from "./abstracts/APIClient";

export default class Gateway extends APIClient {
  constructor(baseUrl: string) {
    super(baseUrl, "/");
  }

  public async getGatewayInformation(): Promise<ClientInterfaces> {
    const response = await this.restClient.sendGet({
      route: `gateway`,
    });
    return response.data;
  }

  public async getGatewayClientInterfaces(): Promise<ClientInterfaces> {
    const response = await this.restClient.sendGet({
      route: `gateway/client-interfaces`,
    });
    return response.data;
  }

  public async getMixnetWebsocketInfo(): Promise<MixnetWebsockets> {
    const response = await this.restClient.sendGet({
      route: `gateway/client-interfaces/mixnet-websockets`,
    });
    return response.data;
  }

  public async getWireguardInfo(): Promise<Wireguard> {
    const response = await this.restClient.sendGet({
      route: `gateway/client-interfaces/wireguard`,
    });
    return response.data;
  }
}
