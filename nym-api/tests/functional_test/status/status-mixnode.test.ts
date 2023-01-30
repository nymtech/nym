import Status from "../../src/endpoints/Status";
import ConfigHandler from "../../src/config/configHandler";

let status: Status;
let config: ConfigHandler;

describe("Get mixnode data", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });

  it("Get a mixnode report", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeStatusReport(identity_key);

    expect(typeof response.last_day).toBe("number");
    expect(typeof response.owner).toBe("string");
  });

  it("Get a mixnode stake saturation", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeStakeSaturation(identity_key);

    expect(typeof response.as_at).toBe("number");
    expect(typeof response.saturation).toBe("string");
    expect(typeof response.uncapped_saturation).toBe("string");
  });

  it("Get a mixnode average uptime", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeAverageUptime(identity_key);

    expect(identity_key).toStrictEqual(response.mix_id);
    expect(typeof response.avg_uptime).toBe("number");
  });

  it("Get a mixnode history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeHistory(identity_key);

    response.history.forEach((x) => {
      console.log(x.date);
      console.log(x.uptime);
    })

    expect(identity_key).toStrictEqual(response.mix_id);
    expect(typeof response.owner).toBe("string");
  });

  it("Get a mixnode core count", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeCoreCount(identity_key);

    expect(identity_key).toStrictEqual(response.mix_id);
    expect(typeof response.count).toBe("number");
  });

  it("Get a mixnode status", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeStatus(identity_key);

    expect(response.status).toStrictEqual("active");
  });

  it("Get a mixnode reward estimation", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeRewardComputation(identity_key);

    expect(response.reward_params.interval.sybil_resistance).toStrictEqual("0.3");
    expect(response.reward_params.active_set_size).toStrictEqual(240);
    expect(typeof response.reward_params.interval.reward_pool).toBe("string");
  });

  it("Get a mixnode inclusion probability", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeInclusionProbability(identity_key);

    expect(typeof response.in_active).toBe("string");
  });

  it("Get all mixnodes inclusion probability", async (): Promise<void> => {
    const response = await status.getAllMixnodeInclusionProbability();

    expect(response.inclusion_probabilities).toBeTruthy();
  });

  it("Get all mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedMixnodes();

    expect(typeof response.stake_saturation).toBe("string");
  });

  it("Get all rewarded mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedRewardedMixnodes();

    expect(typeof response.mixnode_details.rewarding_details.last_rewarded_epoch).toBe("number");
  });

  it("Get all active mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedActiveMixnodes();

    expect(typeof response.mixnode_details.bond_information.layer).toBe("number");
  });

});

describe("Compute mixnode reward estimation", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  }); 
  it("with correct data", async (): Promise<void> => {
    const response = await status.sendMixnodeRewardEstimatedComputation(8);
    const body = 

    expect(typeof response.estimation.total_node_reward).toBe("string");
  });

});
