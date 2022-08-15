import Status from "../../../src/endpoints/Status";
import ConfigHandler from "../../../src/config/configHandler";

let status: Status;
let config: ConfigHandler;

describe("Get gateway data", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });

  it("Get a gateway history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayHistory(identity_key);

    response.history.forEach((x) => {
      console.log(x.date);
      console.log(x.uptime);

      expect(typeof x.date).toBe("string");
      expect(typeof x.uptime).toBe("number");
    });
    console.log(response.identity);
    console.log(response.owner);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.owner).toBe("string");
  });

  it("Get gateway core status count", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayCoreCount(identity_key);

    console.log(response.count);
    console.log(response.identity);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.count).toBe("number");
  });

  it("Get a gateway status report", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayStatusReport(identity_key);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.owner).toBe("string");
    expect(typeof response.most_recent).toBe("number");
    expect(typeof response.last_hour).toBe("number");
    expect(typeof response.last_day).toBe("number");

  });

});