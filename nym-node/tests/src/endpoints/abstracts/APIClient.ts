import { Logger } from "tslog";
import ConfigHandler from "../../../../../common/api-test-utils/config/configHandler";
import { RestClient } from "../../../../../common/api-test-utils/restClient/RestClient";
// import { ConfigHandler, RestClient } from '../../../../../common/api-test-utils';


export abstract class APIClient {
  protected constructor(baseUrl: string, serviceUrl: string) {
    this.url = baseUrl + serviceUrl;
    this.restClient = new RestClient(this.url);
    this.serviceName = this.constructor.toString().match(/\w+/g)[1];
    this.log.info(`The Service URL for ${this.serviceName} is ${this.url}`);
  }

  public createdItemIds: Set<string> = new Set();

  protected config = ConfigHandler.getInstance();

  protected log: Logger = new Logger({
    minLevel: this.config.environmentConfig.log_level,
    dateTimeTimezone:
      this.config.environmentConfig.time_zone ||
      Intl.DateTimeFormat().resolvedOptions().timeZone,
  });

  protected url: string;

  public restClient: RestClient;

  protected serviceName: string;
}
