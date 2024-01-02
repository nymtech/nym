import Status from "../../src/endpoints/Status";
import ConfigHandler from "../../src/config/configHandler";

let status: Status;
let config: ConfigHandler;

describe("Get gateway data", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });

  it("Get all gateways detailed", async (): Promise<void> => {
    const response = await status.getDetailedGateways();
    response.forEach((x) => {
      expect(typeof x.gateway_bond.owner).toBe("string");
      expect(typeof x.performance).toBe("string");
      expect(typeof x.node_performance.last_24h).toBe("string");
    });
  });

  it("Get all gateways unfiltered", async (): Promise<void> => {
    const response = await status.getUnfilteredGateways();
    response.forEach((x) => {
      expect(typeof x.gateway_bond.owner).toBe("string");
      expect(typeof x.performance).toBe("string");
      expect(typeof x.node_performance.last_24h).toBe("string");
    });
  });

  it("Get a gateway history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayHistory(identity_key);

    if ("identity" in response) {
      response.history.forEach((x) => {
        expect(typeof x.date).toBe("string");
        expect(typeof x.uptime).toBe("number");
      });
      expect(identity_key).toStrictEqual(response.identity);
      expect(typeof response.owner).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain(
        "could not find uptime history associated with gateway"
      );
    }
  });

  it("Get gateway core status count", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayCoreCount(identity_key);
    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.count).toBe("number");
  });

  it("Get gateway average uptime", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayAverageUptime(identity_key);
    if ("identity" in response) {
      expect(identity_key).toStrictEqual(response.identity);
      expect(typeof response.avg_uptime).toBe("number");
    } else if ("message" in response) {
      expect(response.message).toContain("gateway bond not found");
    }
  });

  it("Get a gateway status report", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayStatusReport(identity_key);
    if ("identity" in response) {
      expect(identity_key).toStrictEqual(response.identity);
      expect(typeof response.owner).toBe("string");
      expect(typeof response.most_recent).toBe("number");
      expect(typeof response.last_hour).toBe("number");
      expect(typeof response.last_day).toBe("number");
    } else if ("message" in response) {
      expect(response.message).toContain("gateway bond not found");
    }
  });
});
