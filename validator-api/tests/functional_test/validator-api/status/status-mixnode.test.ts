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
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeStakeSaturation(mix_id);

    console.log(response.as_at);
    console.log(response.saturation);

    expect(typeof response.as_at).toBe("number");
    expect(typeof response.saturation).toBe("string");
    expect(typeof response.uncapped_saturation).toBe("string");
  });

  it("Get a mixnode status report", async (): Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeStatusReport(mix_id);

    expect(mix_id).toStrictEqual(response.mix_id);
    expect(typeof response.owner).toBe("string");
    expect(typeof response.most_recent).toBe("number");
    expect(typeof response.last_hour).toBe("number");
    expect(typeof response.last_day).toBe("number");
  });

  it("Get a mixnode average uptime", async (): Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeAverageUptime(mix_id);

    console.log(response.avg_uptime);
    console.log(response.mix_id);

    expect(mix_id).toStrictEqual(response.mix_id);
    expect(typeof response.avg_uptime).toBe("number");
  });


  it("Get a mixnode history", async (): Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeHistory(mix_id);

    response.history.forEach((x) => {
      console.log(x.date);
      console.log(x.uptime);

      expect(typeof x.date).toBe("string");
      expect(typeof x.uptime).toBe("number");
    });
    console.log(response.identity);
    console.log(response.owner);

    expect(mix_id).toStrictEqual(response.mix_id);
    expect(typeof response.owner).toBe("string");
  });

  it("Get mixnode core status count", async (): Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeCoreCount(mix_id);

    console.log(response.count);
    console.log(response.mix_id);

    expect(mix_id).toStrictEqual(response.mix_id);
    expect(typeof response.count).toBe("number");
  });

  it("Get a mixnode status", async (): Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeStatus(mix_id);

    console.log(response.status);

    expect(response.status).toStrictEqual("active");
  });

  it("Get a mixnode reward estimation", async (): Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeRewardComputation(mix_id);

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
    expect(typeof response.reward_params.rewarded_set_size).toBe("number");
    expect(typeof response.reward_params.active_set_size).toBe("number");

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
    const mix_id = config.environmnetConfig.mixnode_identity;
    const response = await status.getMixnodeInclusionProbability(mix_id);

    console.log(response.in_active);
    console.log(response.in_reserve);

    expect(typeof response.in_active).toBe("string");
    expect(typeof response.in_reserve).toBe("string");
  });

  it("Post to compute mixnode reward estimation", async ():Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const payload = {"performance": "0.2"}
    const response = await status.getMixnodeRewardEstimatedComputation(mix_id, payload);

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
    expect(typeof response.reward_params.rewarded_set_size).toBe("number");
    expect(typeof response.reward_params.active_set_size).toBe("number");

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


  it.skip("Post to compute mixnode reward estimation", async ():Promise<void> => {
    const mix_id = config.environmnetConfig.mixnode_identity;
    const payload = {"performance": "0.7"}
    const response = await status.getMixnodeRewardEstimatedComputation(mix_id, payload);

    // TO-DO this test needs calculations to ensure than when passing through different performance values, the reward is also changing as expected
    expect(response.estimation.total_node_reward).toContain("986360");

  })
});
