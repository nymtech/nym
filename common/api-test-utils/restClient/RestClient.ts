
import { Logger } from "tslog";
import { stringify } from "yaml";
import https from "https";
import ConfigHandler from "../config/configHandler";
const { createMixFetch } = require("@nymproject/mix-fetch-node-commonjs");

const config = ConfigHandler.getInstance();
const log = new Logger({
  minLevel: config.environmentConfig.log_level,
  dateTimeTimezone:
    config.environmentConfig.time_zone ||
    Intl.DateTimeFormat().resolvedOptions().timeZone,
});

export class RestClient {
  public static authToken: string;
  private baseUrl: string;
  private mixFetch: any;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
    this.mixFetch = this.initialiseMixFetch
  }


  // MIXFETCH 
  private initialiseMixFetch() {
    const extra = {
      hiddenGateways: [
        {
          owner: "n1ns3v70ul9gnl9l9fkyz8cyxfq75vjcmx8el0t3",
          host: "sandbox-gateway1.nymtech.net",
          explicitIp: "35.158.238.80",
          identityKey: "HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua",
          sphinxKey: "BoXeUD7ERGmzRauMjJD3itVNnQiH42ncUb6kcVLrb3dy",
        },
      ],
    };

    const mixFetchOptions = {
      nymApiUrl: "https://sandbox-nym-api1.nymtech.net/api",
      preferredGateway: "HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua",
      preferredNetworkRequester:
        "AzGdJ4MU78Ex22NEWfeycbN7bt3PFZr1MtKstAdhfELG.GSxnKnvKPjjQm3FdtsgG5KyhP6adGbPHRmFWDH4XfUpP@HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua",
      mixFetchOverride: {
        requestTimeoutMs: 60_000,
      },
      forceTls: true,
      extra,
    };

    const { mixFetch } = await createMixFetch(mixFetchOptions);
    return mixFetch;
  }

  static async getToken(requestHeaders: object) {
    requestHeaders["Authorization"] = `asdf`;
  }

  public async callEndpoint({
    route,
    method,
    authToken,
    headers,
    data,
    additionalConfigs,
    params,
  }: any): Promise<any> {
    let response;
    let responseLog = "Response: ";
    let requestHeaders = headers || {};

    // if authToken is passed in, add it to the request headers
    if (authToken !== undefined) {
      requestHeaders = {
        ...requestHeaders,
        ...{
          Authorization: `Bearer ${authToken}`,
        },
      };
    } else if (!requestHeaders.Authorization) {
      await RestClient.getToken(requestHeaders);
    }

    log.debug(
      RestClient.prepareLogRecord({
        route,
        method,
        headers: requestHeaders,
        data,
        additionalConfigs,
        params,
      }),
    );

    const mixRequestInit: MixRequestInit = {
      method,
      headers: requestHeaders,
      agent: new https.Agent({
        rejectUnauthorized: false,
      }),
      body: data,
      params,
      ...additionalConfigs,
    };

    try {
      const res = await this.mixFetch(`${this.baseUrl}${route}`, mixRequestInit);
      response = res;
      responseLog = `<Success> Status = ${res.status} ${res.statusText}`;
    } catch (error) {
      response = error.response;
      if (response === undefined)
        responseLog = `<Error> Something wrong happened, did not get proper error from the server! (${error.message})`;
      else
        responseLog = `<Error> Status = ${response.status} ${response.statusText}, ${error.message}`;
    }

    log.debug(responseLog);
    return response;
  }

  private static prepareLogRecord({
    route,
    method,
    headers,
    data,
    additionalConfigs,
    params,
  }: any): string {
    let logRecord = `Request: ${method} ${route}`;
    if (headers) logRecord = `${logRecord}\nHeaders: ${stringify(headers)}`;
    if (params) logRecord = `${logRecord}\nParams: ${stringify(params)}`;
    if (additionalConfigs)
      logRecord = `${logRecord}\nAdditional Configuration: ${stringify(
        additionalConfigs,
      )}`;
    if (data) {
      const jsonData = stringify(data);
      logRecord = `${logRecord}\nData: ${
        jsonData === undefined ? "Some data, not JSON!" : jsonData
      }`;
    }
    return logRecord;
  }
}