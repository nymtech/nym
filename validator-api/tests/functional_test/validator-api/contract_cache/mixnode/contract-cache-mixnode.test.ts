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
      expect(typeof mixnode.owner).toBe('string');
      expect(typeof mixnode.block_height).toBe('number');
      expect(typeof mixnode.layer).toBe('number');
      expect(typeof mixnode.accumulated_rewards).toBe('string');

      if (mixnode.proxy === null) {
        return true;
      }
      else {
        expect(typeof mixnode.proxy).toBe('string');
      }

      //pledge 
      expect(typeof mixnode.pledge_amount.amount).toBe('string');
      expect(mixnode.pledge_amount.denom).toBe('unym');

      //total_deleglation
      expect(typeof mixnode.total_delegation.amount).toBe('string')
      expect(mixnode.total_delegation.denom).toBe('unym');

      //mixnode
      expect(typeof mixnode.mix_node.host).toBe('string')
      expect(typeof mixnode.mix_node.profit_margin_percent).toBe('number');

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
      expect(typeof mixnode.mix_node.version).toBe('string');
    });
  });

  it("Get all mixnodes detailed", async (): Promise<void> => {
    const response = await contract.getMixnodesDetailed();
    response.forEach((mixnode) => {
      //mixnode_bond.pledge_amount
      expect(typeof mixnode.mixnode_bond.pledge_amount.amount).toBe('string');
      expect(mixnode.mixnode_bond.pledge_amount.denom).toBe('unym');

      //mixnode_bond.total_delegation
      expect(typeof mixnode.mixnode_bond.total_delegation.amount).toBe('string')
      expect(mixnode.mixnode_bond.total_delegation.denom).toBe('unym');

      //mixnode_bond.mix_node
      expect(typeof mixnode.mixnode_bond.mix_node.host).toBe('string')
      expect(typeof mixnode.mixnode_bond.mix_node.profit_margin_percent).toBe('number');

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
      expect(typeof mixnode.mixnode_bond.mix_node.version).toBe('string');

      //mixnode_bond.overview
      expect(typeof mixnode.mixnode_bond.owner).toBe('string');
      expect(typeof mixnode.mixnode_bond.block_height).toBe('number');
      expect(typeof mixnode.mixnode_bond.layer).toBe('number');
      expect(typeof mixnode.mixnode_bond.accumulated_rewards).toBe('string');

      if (mixnode.mixnode_bond.proxy === null) {
        return true;
      }
      else {
        expect(typeof mixnode.mixnode_bond.proxy).toBe('string');
      }

      //overview
      expect(typeof mixnode.stake_saturation).toBe('number');
      expect(typeof mixnode.uptime).toBe('number');
      expect(typeof mixnode.estimated_operator_apy).toBe('number');
      expect(typeof mixnode.estimated_delegators_apy).toBe('number');
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
      expect(typeof value).toBe('string');
    });
  });

});


