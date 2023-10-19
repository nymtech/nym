import { BuildInformation, HostInformation, Roles } from "../types/NodeTypes";
import { APIClient } from "./abstracts/APIClient";

export default class Nodes extends APIClient {
  constructor(baseUrl: string) {
    super(baseUrl, "/");
  }

  public async getBuildInformation(): Promise<BuildInformation> {
    const response = await this.restClient.sendGet({
      route: `build-information`,
    });
    return response.data;
  }

  public async getHostInformation(): Promise<HostInformation> {
    const response = await this.restClient.sendGet({
      route: `host-information`,
    });
    return response.data;
  }

  public async getSupportedRoles(): Promise<Roles> {
    const response = await this.restClient.sendGet({
      route: `roles`,
    });
    return response.data;
  }
}
