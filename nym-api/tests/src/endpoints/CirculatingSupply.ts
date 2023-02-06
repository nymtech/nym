import { Detailed } from "../types/CirculatingSupplyTypes";
import { APIClient } from "./abstracts/APIClient";

export default class ContractCache extends APIClient {
  constructor() {
    super("/");
  }

  public async getCirculatingSupply(): Promise<Detailed> {
    const response = await this.restClient.sendGet({
      route: `circulating-supply`,
    });
    return response.data;
  }

  public async getTotalSupplyValue(): Promise<number> {
    const response = await this.restClient.sendGet({
      route: `circulating-supply/total-supply-value`,
    });
    return response.data;
  }

  public async getCirculatingSupplyValue(): Promise<number> {
    const response = await this.restClient.sendGet({
      route: `circulating-supply/circulating-supply-value`,
    });
    return response.data;
  }
}
