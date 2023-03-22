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
    if ("mix_id" in response) {
        expect(identity_key).toStrictEqual(response.mix_id);
        expect(typeof response.avg_uptime).toBe("number");
    } else if ("message" in response) {
      expect(response.message).toContain("could not find uptime history associated with mixnode");
    }
  });

  it("Get a mixnode history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mix_id;
    const response = await status.getMixnodeHistory(identity_key);

    if ("mix_id" in response) {
      response.history.forEach((x) => {
        expect(typeof x.date).toBe("string");
        expect(typeof x.uptime).toBe("number");
      });
      expect(identity_key).toStrictEqual(response.mix_id);
      expect(typeof response.owner).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain("could not find uptime history associated with mixnode");
    }
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

  it("Get all mixnodes inclusion probabilities", async (): Promise<void> => {
    const response = await status.getAllMixnodeInclusionProbability();
    expect(typeof response.inclusion_probabilities[0].mix_id).toBe("number");
    expect(typeof response.inclusion_probabilities[0].in_active).toBe("number");
    expect(typeof response.delta_max).toBe("number");
  });

  it("Get all mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedMixnodes();
    expect(typeof response[0].stake_saturation).toBe("string");
  });

  it("Get all rewarded mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedRewardedMixnodes();
    expect(typeof response[0].mixnode_details.rewarding_details.last_rewarded_epoch).toBe("number");
  });

  it("Get all active mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedActiveMixnodes();

    expect(typeof response[0].mixnode_details.bond_information.layer).toBe("number");
  });
});

describe("Compute mixnode reward estimation", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });
  it("with correct data", async (): Promise<void> => {
    const response = await status.sendMixnodeRewardEstimatedComputation(63);

    expect(typeof response.estimation.delegates).toBe("string");
  });
});
