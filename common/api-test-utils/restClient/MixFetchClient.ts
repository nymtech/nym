import ConfigHandler from "../config/configHandler";
import { createMixFetch } from "@nymproject/mix-fetch-node-commonjs";
import * as dotenv from 'dotenv'; 
import path from "path";

dotenv.config({ path: path.join(__dirname, '../.env') });;
const config = ConfigHandler.getInstance();

export class MixFetchClient {
  public static authToken: string;
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  public async sendGet({
    route,
  }: any): Promise<any> {

    const extra = {
      hiddenGateways: [
        {
          owner: process.env.HIDDEN_GATEWAY_OWNER,
          host: process.env.HIDDEN_GATEWAY_HOST,
          explicitIp: process.env.HIDDEN_GATEWAY_EXPLICIT_IP,
          identityKey: process.env.HIDDEN_GATEWAY_IDENTITY_KEY,
          sphinxKey: process.env.HIDDEN_GATEWAY_SPHINX_KEY,
        },
      ],
    };

    const mixFetchOptions = {
      nymApiUrl: process.env.PREFERRED_VALIDATOR,
      preferredGateway: process.env.PREFERRED_GATEWAY,
      preferredNetworkRequester: process.env.PREFFERED_NETWORK_REQUESTER,
      mixFetchOverride: {
        requestTimeoutMs: 60_000,
      },
      forceTls: true,
      extra,
    };

    const { mixFetch } = await createMixFetch(mixFetchOptions);

    let args = {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
      mode: "unsafe-ignore-cors"
    };

    const response = await mixFetch(`${this.baseUrl}${route}`, args);
    const json = await response.json();
    return json;
  };

}