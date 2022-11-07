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
      //bond information overview
      expect(typeof mixnode.bond_information.mix_id).toBe('number');
      expect(typeof mixnode.bond_information.owner).toBe('string');
      expect(typeof mixnode.bond_information.original_pledge.amount).toBe('string');
      expect(typeof mixnode.bond_information.original_pledge.denom).toBe('string');
      expect(typeof mixnode.bond_information.layer).toBe('string');
      expect(typeof mixnode.bond_information.bonding_height).toBe('number');
      expect(typeof mixnode.bond_information.is_unbonding).toBe('boolean');

      if (mixnode.bond_information.proxy === null) {
        return true;
      }
      else {
        expect(typeof mixnode.bond_information.proxy).toBe('string');
      }

      //mixnode
      expect(typeof mixnode.bond_information.mix_node.host).toBe('string')
      expect(typeof mixnode.bond_information.mix_node.http_api_port).toStrictEqual(8000);
      expect(typeof mixnode.bond_information.mix_node.verloc_port).toBe('number')
      expect(typeof mixnode.bond_information.mix_node.mix_port).toBe('number')
      expect(typeof mixnode.bond_information.mix_node.sphinx_key).toBe('number')
      expect(typeof mixnode.bond_information.mix_node.mix_port).toStrictEqual(1789);
      expect(typeof mixnode.bond_information.mix_node.verloc_port).toStrictEqual(1790)

      let identitykey = mixnode.bond_information.mix_node.identity_key
      if (typeof identitykey === 'string') {
        if (identitykey.length === 43) {
          return true
        }
        else expect(identitykey).toHaveLength(44);
      }

      let sphinx = mixnode.bond_information.mix_node.sphinx_key
      if (typeof sphinx === 'string') {
        if (sphinx.length === 43) {
          return true
        }
        else expect(sphinx).toHaveLength(44);
      }

      //rewarding details
      expect(typeof mixnode.rewarding_details.cost_params.profit_margin_percent).toBe('string')
      expect(typeof mixnode.rewarding_details.cost_params.interval_operating_cost.denom).toBe('string')
      expect(typeof mixnode.rewarding_details.cost_params.interval_operating_cost.amount).toBe('string')
      expect(typeof mixnode.rewarding_details.operator).toBe('string')
      expect(typeof mixnode.rewarding_details.delegates).toBe('string')
      expect(typeof mixnode.rewarding_details.total_unit_reward).toBe('string')
      expect(typeof mixnode.rewarding_details.unit_delegation).toBe('string')
      expect(typeof mixnode.rewarding_details.last_rewarded_epoch).toBe('number')
      expect(typeof mixnode.rewarding_details.unique_delegations).toBe('number')

    });
  });

  it("Get all mixnodes detailed", async (): Promise<void> => {
    const response = await contract.getMixnodesDetailed();
    response.forEach((mixnode) => {
      // overview details
      expect(typeof mixnode.estimated_delegators_apy).toBe('string');
      expect(typeof mixnode.estimated_operator_apy).toBe('string');
      expect(typeof mixnode.performance).toBe('string');
      expect(typeof mixnode.uncapped_stake_saturation).toBe('string');
      expect(typeof mixnode.stake_saturation).toBe('string');

      //mixnode details bond info
      expect(typeof mixnode.mixnode_details.bond_information.mix_id).toBe('string')
      expect(typeof mixnode.mixnode_details.bond_information.owner).toBe('string');
      expect(typeof mixnode.mixnode_details.bond_information.original_pledge.amount).toBe('string');
      expect(typeof mixnode.mixnode_details.bond_information.original_pledge.denom).toBe('string');
      expect(typeof mixnode.mixnode_details.bond_information.layer).toBe('string');
      expect(typeof mixnode.mixnode_details.bond_information.bonding_height).toBe('number');
      expect(typeof mixnode.mixnode_details.bond_information.is_unbonding).toBe('boolean');

      if (mixnode.mixnode_details.bond_information.proxy === null) {
        return true;
      }
      else {
        expect(typeof mixnode.mixnode_details.bond_information.proxy).toBe('string');
      }

      //mixnode
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.host).toBe('string')
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.http_api_port).toStrictEqual(8000);
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.verloc_port).toBe('number')
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.mix_port).toBe('number')
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.sphinx_key).toBe('number')
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.mix_port).toStrictEqual(1789);
      expect(typeof mixnode.mixnode_details.bond_information.mix_node.verloc_port).toStrictEqual(1790)

      let identitykey2 = mixnode.mixnode_details.bond_information.mix_node.identity_key
      if (typeof identitykey2 === 'string') {
        if (identitykey2.length === 43) {
          return true
        }
        else expect(identitykey2).toHaveLength(44);
      }

      let sphinx2 = mixnode.mixnode_details.bond_information.mix_node.sphinx_key
      if (typeof sphinx2 === 'string') {
        if (sphinx2.length === 43) {
          return true
        }
        else expect(sphinx2).toHaveLength(44);
      }

      //mixnode rewarding info
      expect(typeof mixnode.mixnode_details.rewarding_details.cost_params.profit_margin_percent).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.cost_params.interval_operating_cost.denom).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.cost_params.interval_operating_cost.amount).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.operator).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.delegates).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.total_unit_reward).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.unit_delegation).toBe('string')
      expect(typeof mixnode.mixnode_details.rewarding_details.last_rewarded_epoch).toBe('number')
      expect(typeof mixnode.mixnode_details.rewarding_details.unique_delegations).toBe('number')

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
      expect(typeof value).toBe('number');
    });
  });

});
