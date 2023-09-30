import { NetworkDetails, NymContracts, NymContractsDetailed } from "../types/NetworkTypes";
import { APIClient } from "./abstracts/APIClient";

export default class NetworkTypes extends APIClient {
  constructor() {
    super("/");
  }

  public async getNetworkDetails(): Promise<NetworkDetails> {
    const response = await this.restClient.sendGet({
      route: `network/details`,
    });
    return response.data;
  }

  public async getNymContractInfo(): Promise<NymContracts> {
    const response = await this.restClient.sendGet({
      route: `network/nym-contracts`,
    });
    return response.data;
  }

  public async getNymContractDetailedInfo(): Promise<NymContractsDetailed> {
    const response = await this.restClient.sendGet({
      route: `network/nym-contracts-detailed`,
    });
    return response.data;
  }
}
