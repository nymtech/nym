import ContractCache from "../../../../src/endpoints/ContractCache";
import ConfigHandler from "../../../../src/config/configHandler";

let contract: ContractCache;
let config: ConfigHandler;


describe("Get mixnode data", (): void => {
  beforeAll(async (): Promise<void> => {
    contract = new ContractCache();
    config = ConfigHandler.getInstance();
  });

  it("Get all mixnodes", async (): Promise<void> => {
    const response = await contract.getMixnodes();

    response.forEach((mixnode) => {
      //overview
      expect(typeof mixnode.owner).toStrictEqual('string');
      expect(typeof mixnode.block_height).toStrictEqual('number');
      expect(typeof mixnode.layer).toStrictEqual('number');
      expect(typeof mixnode.accumulated_rewards).toStrictEqual('string');

      if (typeof mixnode.proxy === null) {
        return;
      }
      else {
        expect(typeof mixnode.proxy).toStrictEqual('string');
      }

      //pledge 
      expect(typeof mixnode.pledge_amount.amount).toStrictEqual('string');
      expect(mixnode.pledge_amount.denom).toStrictEqual('unym');

      //total_deleglation
      expect(typeof mixnode.total_delegation.amount).toStrictEqual('string')
      expect(mixnode.total_delegation.denom).toStrictEqual('unym');

      //mixnode
      expect(typeof mixnode.mix_node.host).toStrictEqual('string')
      expect(typeof mixnode.mix_node.profit_margin_percent).toStrictEqual('number');

      let identitykey = mixnode.mix_node.identity_key
      if (typeof identitykey === 'string') {
        if (identitykey.length === 43) {
          return true
        }
        else expect(identitykey).toHaveLength(44);
      }

      let sphinx = mixnode.mix_node.sphinx_key
      if (typeof sphinx === 'string') {
        if (sphinx.length === 43) {
          return true
        }
        else expect(sphinx).toHaveLength(44);
      }
      expect(mixnode.mix_node.verloc_port).toStrictEqual(1790);
      expect(mixnode.mix_node.mix_port).toStrictEqual(1789);
      expect(mixnode.mix_node.http_api_port).toStrictEqual(8000);
      expect(typeof mixnode.mix_node.version).toStrictEqual('string');
    });
  });

  it("Get all mixnodes detailed", async (): Promise<void> => {
    const response = await contract.getMixnodesDetailed();
    response.forEach((mixnode) => {
      //mixnode_bond.pledge_amount
      expect(typeof mixnode.mixnode_bond.pledge_amount.amount).toStrictEqual('string');
      expect(mixnode.mixnode_bond.pledge_amount.denom).toStrictEqual('unym');

      //mixnode_bond.total_delegation
      expect(typeof mixnode.mixnode_bond.total_delegation.amount).toStrictEqual('string')
      expect(mixnode.mixnode_bond.total_delegation.denom).toStrictEqual('unym');

      //mixnode_bond.mix_node
      expect(typeof mixnode.mixnode_bond.mix_node.host).toStrictEqual('string')
      expect(typeof mixnode.mixnode_bond.mix_node.profit_margin_percent).toStrictEqual('number');

      let identitykey = mixnode.mixnode_bond.mix_node.identity_key
      if (typeof identitykey === 'string') {
        if (identitykey.length === 43) {
          return true
        }
        else expect(identitykey).toHaveLength(44);
      }

      let sphinx = mixnode.mixnode_bond.mix_node.sphinx_key
      if (typeof sphinx === 'string') {
        if (sphinx.length === 43) {
          return true
        }
        else expect(sphinx).toHaveLength(44);
      }

      expect(mixnode.mixnode_bond.mix_node.verloc_port).toStrictEqual(1790);
      expect(mixnode.mixnode_bond.mix_node.mix_port).toStrictEqual(1789);
      expect(mixnode.mixnode_bond.mix_node.http_api_port).toStrictEqual(8000);
      expect(typeof mixnode.mixnode_bond.mix_node.version).toStrictEqual('string');

      //mixnode_bond.overview
      expect(typeof mixnode.mixnode_bond.owner).toStrictEqual('string');
      expect(typeof mixnode.mixnode_bond.block_height).toStrictEqual('number');
      expect(typeof mixnode.mixnode_bond.layer).toStrictEqual('number');
      expect(typeof mixnode.mixnode_bond.accumulated_rewards).toStrictEqual('string');

      if (typeof mixnode.mixnode_bond.proxy === null) {
        return;
      }
      else {
        expect(typeof mixnode.mixnode_bond.proxy).toStrictEqual('string');
      }

      //overview
      expect(typeof mixnode.stake_saturation).toStrictEqual('number');
      expect(typeof mixnode.uptime).toStrictEqual('number');
      expect(typeof mixnode.estimated_operator_apy).toStrictEqual('number');
      expect(typeof mixnode.estimated_delegators_apy).toStrictEqual('number');
    });
  });

  it("Get all gateways", async (): Promise<void> => {
    const response = await contract.getGateways();
    response.forEach((gateway) => {
      //overview
      expect(typeof gateway.owner).toStrictEqual('string');
      expect(typeof gateway.block_height).toStrictEqual('number');

      if (typeof gateway.proxy === null) {
        return;
      }
      else {
        expect(typeof gateway.proxy).toStrictEqual('string');
      }

      //pledge_amount
      expect(typeof gateway.pledge_amount.denom).toStrictEqual('string');
      expect(typeof gateway.pledge_amount.amount).toStrictEqual('string');

      //gateway
      expect(typeof gateway.gateway.host).toStrictEqual('string');
      expect(typeof gateway.gateway.mix_port).toStrictEqual('number');
      expect(typeof gateway.gateway.clients_port).toStrictEqual('number');
      expect(typeof gateway.gateway.location).toStrictEqual('string');
      expect(typeof gateway.gateway.sphinx_key).toStrictEqual('string');
      expect(typeof gateway.gateway.identity_key).toStrictEqual('string');
      expect(typeof gateway.gateway.version).toStrictEqual('string');
    });
  });

  it("Get active mixnodes", async (): Promise<void> => {
    const response = await contract.getActiveMixnodes();
    //TO-DO this test should focus more on checking that the response actually contains active nodes
  });

  //TO-DO figure out a similar type of test solution as above for the following: 
  // getActiveMixnodesDetailed
  // getRewardedMixnodes
  // getRewardedMixnodesDetailed

  it("Get blacklisted mixnodes", async (): Promise<void> => {
    const response = await contract.getBlacklistedMixnodes();
    response.forEach(function (value) {
      expect(typeof value).toStrictEqual('string');
    });
  });

  it("Get blacklisted gateways", async (): Promise<void> => {
    const response = await contract.getBlacklistedGateways();
    response.forEach(function (value) {
      expect(typeof value).toStrictEqual('string');
    });
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


