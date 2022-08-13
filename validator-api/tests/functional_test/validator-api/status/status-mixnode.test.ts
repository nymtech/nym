import Status from "../../../src/endpoints/Status";
import ConfigHandler from "../../../src/config/configHandler";

let status: Status;
let config: ConfigHandler;

describe.skip("Get mixnode data", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });

  it("Get a mixnode stake saturation", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeStakeSaturation(identity_key);

    console.log(response.as_at);
    console.log(response.saturation);

    expect(typeof response.as_at).toBe("number");
    expect(typeof response.saturation).toBe("number");
  });

  it("Get a mixnode average uptime", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeAverageUptime(identity_key);

    console.log(response.avg_uptime);
    console.log(response.identity);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.avg_uptime).toBe("number");
  });

  it("Get a mixnode history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeHistory(identity_key);

    response.history.forEach((x) => {
      console.log(x.date);
      console.log(x.uptime);
    });
    console.log(response.identity);
    console.log(response.owner);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.owner).toBe("string");
  });

  it("Get a gateway history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayHistory(identity_key);

    response.history.forEach((x) => {
      console.log(x.date);
      console.log(x.uptime);
    });
    console.log(response.identity);
    console.log(response.owner);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.owner).toBe("string");
  });

  it("Get a gateway history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.gateway_identity;
    const response = await status.getGatewayCoreCount(identity_key);

    console.log(response.count);
    console.log(response.identity);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.count).toBe("number");
  });

  it("Get a gateway history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeCoreCount(identity_key);

    console.log(response.count);
    console.log(response.identity);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.count).toBe("number");
  });

  it("Get a mixnode status", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeStatus(identity_key);

    console.log(response.status);

    expect(response.status).toStrictEqual("active");
  });

  it("Get a mixnode reward estimation", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeRewardComputation(identity_key);

    console.log(response.estimated_delegators_reward);
    console.log(response.estimated_node_profit);
    console.log(response.estimated_operator_cost);
    console.log(response.estimated_operator_reward);
    console.log(response.estimated_total_node_reward);
    console.log(response.reward_params);
    console.log(response.as_at);
    console.log(response);

    //assertions to come
    //expect(response).toStrictEqual('something');
  });

  it("Get a mixnode inclusion probability", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeInclusionProbability(identity_key);

    console.log(response.in_active);
    console.log(response.in_reserve);

    //assertions to come
    //expect(response).toStrictEqual('something');
  });
});
