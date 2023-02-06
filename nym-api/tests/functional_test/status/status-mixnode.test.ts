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
      expect(typeof x.date).toBe("string");
      expect(typeof x.uptime).toBe("number");
    });

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

    expect(response.reward_params.interval.sybil_resistance).toStrictEqual(
      "0.3"
    );
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
    const array = response.inclusion_probabilities;
    array.forEach((x) => {
      expect(typeof x.in_reserve).toBe("number");
      expect(typeof x.mix_id).toBe("number");
    });
    expect(typeof response.elapsed.nanos).toBe("number");
  });

  it("Get all mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedMixnodes();
    response.forEach((x) => {
      expect(typeof x.mixnode_details.bond_information.mix_id).toBe("number");
      expect(typeof x.mixnode_details.bond_information.layer).toBe("number");
      expect(typeof x.stake_saturation).toBe("string");
    });
  });

  it("Get all rewarded mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedRewardedMixnodes();
    response.forEach((x) => {
      expect(typeof x.mixnode_details.bond_information.mix_id).toBe("number");
      expect(typeof x.mixnode_details.bond_information.layer).toBe("number");
      expect(typeof x.stake_saturation).toBe("string");
    });
  });

  it("Get all active mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedActiveMixnodes();
    response.forEach((x) => {
      expect(typeof x.mixnode_details.bond_information.mix_id).toBe("number");
      expect(typeof x.mixnode_details.bond_information.layer).toBe("number");
      expect(typeof x.stake_saturation).toBe("string");
    });
  });
});

describe("Compute mixnode reward estimation", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });
  // TODO Fix this test
  it.skip("with correct data", async (): Promise<void> => {
    const response = await status.sendMixnodeRewardEstimatedComputation(8);
    const body = expect(typeof response.estimation.total_node_reward).toBe(
      "string"
    );
  });
});
