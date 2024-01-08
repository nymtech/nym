import Status from "../../src/endpoints/Status";
import ConfigHandler from "../../../../common/api-test-utils/config/configHandler"

let status: Status;
let config: ConfigHandler;

describe("Get mixnode data", (): void => {
  beforeAll(async (): Promise<void> => {
    status = new Status();
    config = ConfigHandler.getInstance();
  });

  it("Get a mixnode report", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeStatusReport(identity_key);
    if ("mix_id" in response) {
      expect(typeof response.last_day).toBe("number");
      expect(typeof response.owner).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain("mixnode bond not found");
    }
  });

  it("Get a mixnode stake saturation", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeStakeSaturation(identity_key);
    if ("saturation" in response) {
      expect(typeof response.as_at).toBe("number");
      expect(typeof response.saturation).toBe("string");
      expect(typeof response.uncapped_saturation).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain("mixnode bond not found");
    }
  });

  it("Get a mixnode average uptime", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeAverageUptime(identity_key);
    if ("mix_id" in response) {
      expect(identity_key).toStrictEqual(response.mix_id);
      expect(typeof response.avg_uptime).toBe("number");
    } else if ("message" in response) {
      expect(response.message).toContain("mixnode bond not found");
    }
  });

  it("Get a mixnode history", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeHistory(identity_key);
    if ("mix_id" in response) {
      response.history.forEach((x) => {
        expect(typeof x.date).toBe("string");
        expect(typeof x.uptime).toBe("number");
      });
      expect(identity_key).toStrictEqual(response.mix_id);
      expect(typeof response.owner).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain(
        "could not find uptime history associated with mixnode"
      );
    }
  });

  it("Get a mixnode core count", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeCoreCount(identity_key);
    expect(identity_key).toStrictEqual(response.mix_id);
    expect(typeof response.count).toBe("number");
  });

  it("Get a mixnode reward estimation", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeRewardComputation(identity_key);
    if ("estimation" in response) {
      expect(response.reward_params.interval.sybil_resistance).toStrictEqual(
        "0.3"
      );
      expect(response.reward_params.active_set_size).toStrictEqual(240);
      expect(typeof response.reward_params.interval.reward_pool).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain("mixnode bond not found");
    }
  });

  it("Get a mixnode inclusion probability", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeInclusionProbability(identity_key);
    if ("mix_id" in response) {
      expect(typeof response.in_active).toBe("string");
    } else if ("message" in response) {
      expect(response.message).toContain("mixnode bond not found");
    }
  });

  it("Get all mixnodes inclusion probabilities", async (): Promise<void> => {
    const response = await status.getAllMixnodeInclusionProbability();
    response.inclusion_probabilities.forEach((x) => {
      expect(typeof x.mix_id).toBe("number");
      expect(typeof x.in_active).toBe("number");
    });
    expect(typeof response.delta_max).toBe("number");
  });

  it("Get all mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedMixnodes();
    response.forEach((x) => {
      expect(typeof x.stake_saturation).toBe("string");
    });
  });

  it("Get all mixnodes unfiltered", async (): Promise<void> => {
    const response = await status.getUnfilteredMixnodes();
    response.forEach((x) => {
      expect(typeof x.stake_saturation).toBe("string");
      expect(
        typeof x.mixnode_details.rewarding_details.last_rewarded_epoch
      ).toBe("number");
    });
  });

  it("Get a mixnode status", async (): Promise<void> => {
    const identity_key = config.environmentConfig.mix_id;
    const response = await status.getMixnodeStatus(identity_key);
    const unfiltered_mixnodes_response = await status.getUnfilteredMixnodes();
    const mixnode = unfiltered_mixnodes_response.find(
      (x) => x.mixnode_details.bond_information.mix_id === identity_key
    );
    if (mixnode) {
      expect(response.status).toStrictEqual("active");
    } else {
      expect(response.status).toStrictEqual("not_found");
    }
  }, 7000);

  it("Get all rewarded mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedRewardedMixnodes();
    response.forEach((x) => {
      expect(
        typeof x.mixnode_details.rewarding_details.last_rewarded_epoch
      ).toBe("number");
    });
  });

  it("Get all active mixnodes", async (): Promise<void> => {
    const response = await status.getDetailedActiveMixnodes();
    response.forEach((x) => {
      expect(typeof x.mixnode_details.bond_information.layer).toBe("number");
    });
  });

  describe("Compute mixnode reward estimation", (): void => {
    beforeAll(async (): Promise<void> => {
      status = new Status();
      config = ConfigHandler.getInstance();
    });

    it("with correct data", async (): Promise<void> => {
      const mix_id = config.environmentConfig.mix_id;
      const response = await status.sendMixnodeRewardEstimatedComputation(
        mix_id
      );
      expect(typeof response.estimation.delegates).toBe("string");
    });
  });
});
