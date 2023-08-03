import ContractCache from "../../src/endpoints/ContractCache";

let contract: ContractCache;

describe("Get epoch info", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
  });

  it("Get epoch reward params", async (): Promise<void> => {
    const response = await contract.getEpochRewardParams();
    expect(typeof response.interval.reward_pool).toBe("string");
    expect(typeof response.interval.staking_supply_scale_factor).toBe("string");
    expect(typeof response.interval.staking_supply).toBe("string");
    expect(typeof response.interval.epoch_reward_budget).toBe("string");
    expect(typeof response.interval.stake_saturation_point).toBe("string");
    expect(typeof response.interval.sybil_resistance).toBe("string");
    expect(typeof response.interval.active_set_work_factor).toBe("string");
    expect(typeof response.interval.interval_pool_emission).toBe("string");
    expect(typeof response.active_set_size).toBe("number");
    expect(typeof response.rewarded_set_size).toBe("number");
  });

  it("Get current epoch", async (): Promise<void> => {
    const response = await contract.getCurrentEpoch();
    expect(typeof response.id).toBe("number");
    expect(typeof response.epochs_in_interval).toBe("number");
    expect(typeof response.current_epoch_id).toBe("number");
    expect(typeof response.current_epoch_start).toBe("string");
    expect(typeof response.epoch_length.secs).toBe("number");
    expect(typeof response.epoch_length.nanos).toBe("number");
    expect(typeof response.total_elapsed_epochs).toBe("number");
  });
});
