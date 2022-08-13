import ContractCache from "../../../src/endpoints/ContractCache";
import ConfigHandler from "../../../src/config/configHandler";

let contract: ContractCache;
let config: ConfigHandler;


describe("Get epoch info", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
    config = ConfigHandler.getInstance();
  });

  it("Get epoch reward params", async (): Promise<void> => {
    const response = await contract.getEpochRewardParams();
    expect(typeof response.epoch_reward_pool).toStrictEqual('string');
    expect(typeof response.rewarded_set_size).toStrictEqual('string');
    expect(typeof response.active_set_size).toStrictEqual('string');
    expect(typeof response.staking_supply).toStrictEqual('string');
    expect(typeof response.sybil_resistance_percent).toStrictEqual('number');
    expect(typeof response.active_set_work_factor).toStrictEqual('number');
  });

  it("Get current epoch", async (): Promise<void> => {
    const response = await contract.getCurrentEpoch();
    expect(typeof response.id).toStrictEqual('number');
    expect(typeof response.start).toStrictEqual('string');
    expect(typeof response.length.secs).toStrictEqual('number');
    expect(typeof response.length.nanos).toStrictEqual('number');
  });


});