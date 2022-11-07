import Status from "../../../src/endpoints/Status";
import ConfigHandler from "../../../src/config/configHandler";

let status: Status;
let config: ConfigHandler;

describe("Get mixnode data", (): void => {
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

  it("Get a mixnode status report", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeStatusReport(identity_key);

    expect(identity_key).toStrictEqual(response.identity);
    expect(typeof response.owner).toBe("string");
    expect(typeof response.most_recent).toBe("number");
    expect(typeof response.last_hour).toBe("number");
    expect(typeof response.last_day).toBe("number");
  });

  it("Get a mixnode average uptime", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeAverageUptime(identity_key);

    console.log(response.avg_uptime);
    console.log(response.mix_id);

    expect(identity_key).toStrictEqual(response.mix_id);
    expect(typeof response.avg_uptime).toBe("number");
  });

  it("Get all mixnodes average uptime", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getAllMixnodeAverageUptime(identity_key);

    response.forEach((mixnode) => {
      expect(typeof mixnode.avg_uptime).toBe("number");
      expect(typeof mixnode.mix_id).toBe("string");
    });
  });

  it("Get a mixnode history", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeHistory(identity_key);

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

  it("Get mixnode core status count", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeCoreCount(identity_key);

    console.log(response.count);
    console.log(response.mix_id);

    expect(identity_key).toStrictEqual(response.mix_id);
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

    //estimation
    expect(typeof response.estimation.total_node_reward).toBe("string");
    expect(typeof response.estimation.operator).toBe("string");
    expect(typeof response.estimation.delegates).toBe("string");
    expect(typeof response.estimation.operating_cost).toBe("string");

    //reward_params
    expect(typeof response.reward_params.interval.reward_pool).toBe("string");
    expect(typeof response.reward_params.interval.staking_supply).toBe("string");
    expect(typeof response.reward_params.interval.staking_supply_scale_factor).toBe("string");
    expect(typeof response.reward_params.interval.epoch_reward_budget).toBe("string");
    expect(typeof response.reward_params.interval.stake_saturation_point).toBe("string");
    expect(typeof response.reward_params.interval.sybil_resistance).toBe("string");
    expect(typeof response.reward_params.interval.active_set_work_factor).toBe("string");
    expect(typeof response.reward_params.interval.interval_pool_emission).toBe("string");
    expect(typeof response.reward_params.rewarded_set_size).toBe("string");
    expect(typeof response.reward_params.active_set_size).toBe("string");

    //epoch
    expect(typeof response.epoch.id).toBe("number");
    expect(typeof response.epoch.epochs_in_interval).toBe("number");
    expect(typeof response.epoch.current_epoch_start).toBe("string");
    expect(typeof response.epoch.current_epoch_id).toBe("number");
    expect(typeof response.epoch.epoch_length.secs).toBe("number");
    expect(typeof response.epoch.epoch_length.nanos).toBe("number");
    expect(typeof response.epoch.total_elapsed_epochs).toBe("number");

    expect(typeof response.as_at).toBe("number");


  });

  it("Get a mixnode inclusion probability", async (): Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeInclusionProbability(identity_key);

    console.log(response.in_active);
    console.log(response.in_reserve);

    expect(typeof response.in_active).toBe("string");
    expect(typeof response.in_reserve).toBe("string");
  });

  it("Post to compute mixnode reward estimation", async ():Promise<void> => {
    const identity_key = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeRewardEstimatedComputation(identity_key);

    //estimation
    expect(typeof response.estimation.total_node_reward).toBe("string");
    expect(typeof response.estimation.operator).toBe("string");
    expect(typeof response.estimation.delegates).toBe("string");
    expect(typeof response.estimation.operating_cost).toBe("string");

    //reward_params
    expect(typeof response.reward_params.interval.reward_pool).toBe("string");
    expect(typeof response.reward_params.interval.staking_supply).toBe("string");
    expect(typeof response.reward_params.interval.staking_supply_scale_factor).toBe("string");
    expect(typeof response.reward_params.interval.epoch_reward_budget).toBe("string");
    expect(typeof response.reward_params.interval.stake_saturation_point).toBe("string");
    expect(typeof response.reward_params.interval.sybil_resistance).toBe("string");
    expect(typeof response.reward_params.interval.active_set_work_factor).toBe("string");
    expect(typeof response.reward_params.interval.interval_pool_emission).toBe("string");
    expect(typeof response.reward_params.rewarded_set_size).toBe("string");
    expect(typeof response.reward_params.active_set_size).toBe("string");

    //epoch
    expect(typeof response.epoch.id).toBe("number");
    expect(typeof response.epoch.epochs_in_interval).toBe("number");
    expect(typeof response.epoch.current_epoch_start).toBe("string");
    expect(typeof response.epoch.current_epoch_id).toBe("number");
    expect(typeof response.epoch.epoch_length.secs).toBe("number");
    expect(typeof response.epoch.epoch_length.nanos).toBe("number");
    expect(typeof response.epoch.total_elapsed_epochs).toBe("number");

    expect(typeof response.as_at).toBe("number");
  })
});
