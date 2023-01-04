import { Logger } from "tslog";
import ConfigHandler from "../../config/configHandler";
import { RestClient } from "../../restClient/RestClient";

export abstract class APIClient {
  protected constructor(serviceUrl: string) {
    const baseUrl: string = this.config.environmnetConfig.api_base_url;
    this.url = baseUrl + serviceUrl;
    this.restClient = new RestClient(this.url);
    this.serviceName = this.constructor.toString().match(/\w+/g)[1];
    this.log.info(`The Service URL for ${this.serviceName} is ${this.url}`);
  }

  public createdItemIds: Set<string> = new Set();

  protected config = ConfigHandler.getInstance();

  protected log: Logger = new Logger({
    minLevel: this.config.environmnetConfig.log_level,
    dateTimeTimezone:
      this.config.environmnetConfig.time_zone ||
      Intl.DateTimeFormat().resolvedOptions().timeZone,
  });

  protected url: string;

  public restClient: RestClient;

  protected serviceName: string;
}
