import axios, {
  AxiosInstance,
  AxiosRequestConfig,
  AxiosResponse,
  Method,
} from "axios";
import { Logger } from "tslog";
import { stringify } from "yaml";

import https from "https";

import ConfigHandler from "../config/configHandler";

const config = ConfigHandler.getInstance();
const log = new Logger({
  minLevel: config.environmentConfig.log_level,
  dateTimeTimezone:
    config.environmentConfig.time_zone ||
    Intl.DateTimeFormat().resolvedOptions().timeZone,
});

function isSet(property): boolean {
  return property !== undefined && property !== null;
}

export class RestClient {
  public static authToken: string;

  private axiosInstance: AxiosInstance;

  constructor(baseUrl: string) {
    this.axiosInstance = axios.create({ baseURL: baseUrl });
  }

  private httpsAgent = new https.Agent({
    rejectUnauthorized: false,
  });

  // Not returning an actual auth token for this example project.
  // Just showing how it can be done!
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
  }: IAxiosCallEndpointArgs): Promise<AxiosResponse> {
    let response;
    let responseLog = "Response: ";
    let requestHeaders = headers;

    // if headers are not passed in, use the default headers
    if (requestHeaders == undefined) {
      requestHeaders = { ...config.commonConfig.request_headers };
    }

    // if authToken is passed in, add it to the request headers
    if (authToken !== undefined) {
      requestHeaders = {
        ...requestHeaders,
        ...{
          Authorization: `Bearer ${authToken}`,
        },
      };
    }

    // if we have not set the auth headers yet, set them
    else if (!requestHeaders.Authorization) {
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

    await this.axiosInstance
      .request({
        url: route,
        method,
        data,
        headers: requestHeaders,
        httpsAgent: this.httpsAgent,
        params,
        ...additionalConfigs,
      })
      .then((res) => {
        response = res;
        responseLog = `<Success> Status = ${res.status} ${res.statusText}`;
      })
      .catch((error) => {
        response = error.response;
        if (response === undefined)
          responseLog = `<Error> Something wrong happened, did not get proper error from server! (${error.message})`;
        else
          responseLog = `<Error> Status = ${response.status} ${response.statusText}, ${error.message}`;
      });
    log.debug(responseLog);
    return response;
  }

  public async sendPost({
    route,
    authToken,
    data,
    params,
    headers,
    additionalConfigs,
  }: IAxiosHttpRequestArgs): Promise<any> {
    return this.callEndpoint({
      route,
      method: "POST",
      authToken,
      data,
      params,
      headers,
      additionalConfigs,
    });
  }

  public async sendGet({
    route,
    authToken,
    params,
    headers,
    additionalConfigs,
  }: IAxiosHttpRequestArgs): Promise<any> {
    return this.callEndpoint({
      route,
      method: "GET",
      authToken,
      params,
      headers,
      additionalConfigs,
    });
  }

  public async sendDelete({
    route,
    authToken,
    params,
    headers,
    additionalConfigs,
  }: IAxiosHttpRequestArgs): Promise<any> {
    return this.callEndpoint({
      route,
      method: "DELETE",
      authToken,
      params,
      headers,
      additionalConfigs,
    });
  }

  public async sendPatch({
    route,
    authToken,
    data,
    headers,
    additionalConfigs,
  }: IAxiosHttpRequestArgs): Promise<any> {
    return this.callEndpoint({
      route,
      method: "PATCH",
      authToken,
      data,
      headers,
      additionalConfigs,
    });
  }

  public async sendPut({
    route,
    authToken,
    data,
    headers,
    additionalConfigs,
  }: IAxiosHttpRequestArgs): Promise<any> {
    return this.callEndpoint({
      route,
      method: "PUT",
      authToken,
      data,
      headers,
      additionalConfigs,
    });
  }

  private static prepareLogRecord({
    route,
    method,
    headers,
    data,
    additionalConfigs,
    params,
  }: IAxiosCallEndpointArgs): string {
    let logRecord = `Request: ${method} ${route}`;
    if (isSet(headers))
      logRecord = `${logRecord}\nHeaders: ${stringify(headers)}`;

    if (isSet(params)) logRecord = `${logRecord}\nParams: ${stringify(params)}`;

    if (isSet(additionalConfigs)) {
      logRecord = `${logRecord}\nAdditional Configuration: ${stringify(
        additionalConfigs,
      )}`;
    }

    if (isSet(data)) {
      const jsonData = stringify(data);
      // We don't want to log anything that isn't json data
      logRecord = `${logRecord}\nData: ${
        jsonData === undefined ? "Some data, not JSON!" : jsonData
      }`;
    }
    return logRecord;
  }
}

export interface IAxiosHttpRequestArgs {
  route: string;
  authToken?: string;
  data?: object;
  params?: object;
  headers?: any;
  additionalConfigs?: AxiosRequestConfig;
}

export interface IAxiosCallEndpointArgs extends IAxiosHttpRequestArgs {
  method: Method;
}
