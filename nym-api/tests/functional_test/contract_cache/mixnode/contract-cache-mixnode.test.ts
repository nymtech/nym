import ContractCache from "../../../src/endpoints/ContractCache";
import ConfigHandler from "../../../../../common/api-test-utils/config/configHandler"

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
      expect(typeof mixnode.bond_information.mix_id).toBe("number");
      expect(typeof mixnode.bond_information.owner).toBe("string");
      expect(typeof mixnode.bond_information.original_pledge.amount).toBe(
        "string"
      );
      expect(typeof mixnode.bond_information.original_pledge.denom).toBe(
        "string"
      );
      expect(typeof mixnode.bond_information.layer).toBe("number");
      expect(typeof mixnode.bond_information.bonding_height).toBe("number");
      expect(typeof mixnode.bond_information.is_unbonding).toBe("boolean");

      if (mixnode.bond_information.proxy === null) {
        return true;
      } else {
        expect(typeof mixnode.bond_information.proxy).toBe("string");
      }

      //mixnode
      expect(typeof mixnode.bond_information.mix_node.host).toBe("string");
      expect(mixnode.bond_information.mix_node.http_api_port).toStrictEqual(
        8000
      );
      expect(typeof mixnode.bond_information.mix_node.verloc_port).toBe(
        "number"
      );
      expect(typeof mixnode.bond_information.mix_node.mix_port).toBe("number");
      expect(mixnode.bond_information.mix_node.mix_port).toStrictEqual(1789);
      expect(typeof mixnode.bond_information.mix_node.verloc_port).toBe(
        "number"
      );

      const identitykey = mixnode.bond_information.mix_node.identity_key;
      if (typeof identitykey === "string") {
        if (identitykey.length === 43) {
          return true;
        } else expect(identitykey).toHaveLength(44);
      }

      const sphinx = mixnode.bond_information.mix_node.sphinx_key;
      if (typeof sphinx === "string") {
        if (sphinx.length === 43) {
          return true;
        } else expect(sphinx).toHaveLength(44);
      }

      //rewarding details
      expect(
        typeof mixnode.rewarding_details.cost_params.profit_margin_percent
      ).toBe("string");
      expect(
        typeof mixnode.rewarding_details.cost_params.interval_operating_cost
          .denom
      ).toBe("string");
      expect(
        typeof mixnode.rewarding_details.cost_params.interval_operating_cost
          .amount
      ).toBe("string");
      expect(typeof mixnode.rewarding_details.operator).toBe("string");
      expect(typeof mixnode.rewarding_details.delegates).toBe("string");
      expect(typeof mixnode.rewarding_details.total_unit_reward).toBe("string");
      expect(typeof mixnode.rewarding_details.unit_delegation).toBe("string");
      expect(typeof mixnode.rewarding_details.last_rewarded_epoch).toBe(
        "number"
      );
      expect(typeof mixnode.rewarding_details.unique_delegations).toBe(
        "number"
      );
    });
  });

  it("Get all mixnodes detailed", async (): Promise<void> => {
    const response = await contract.getMixnodesDetailed();
    response.forEach((mixnode) => {
      // overview details
      expect(typeof mixnode.estimated_delegators_apy).toBe("string");
      expect(typeof mixnode.estimated_operator_apy).toBe("string");
      expect(typeof mixnode.performance).toBe("string");
      expect(typeof mixnode.uncapped_stake_saturation).toBe("string");
      expect(typeof mixnode.stake_saturation).toBe("string");
      // TODO why family is tempermental
      // expect(typeof mixnode.family).toBe("string");

      //mixnode details bond info
      expect(typeof mixnode.mixnode_details.bond_information.mix_id).toBe(
        "number"
      );
      expect(typeof mixnode.mixnode_details.bond_information.owner).toBe(
        "string"
      );
      expect(
        typeof mixnode.mixnode_details.bond_information.original_pledge.amount
      ).toBe("string");
      expect(
        typeof mixnode.mixnode_details.bond_information.original_pledge.denom
      ).toBe("string");
      expect(typeof mixnode.mixnode_details.bond_information.layer).toBe(
        "number"
      );
      expect(
        typeof mixnode.mixnode_details.bond_information.bonding_height
      ).toBe("number");
      expect(typeof mixnode.mixnode_details.bond_information.is_unbonding).toBe(
        "boolean"
      );

      if (mixnode.mixnode_details.bond_information.proxy === null) {
        return true;
      } else {
        expect(typeof mixnode.mixnode_details.bond_information.proxy).toBe(
          "string"
        );
      }

      //mixnode
      expect(
        typeof mixnode.mixnode_details.bond_information.mix_node.host
      ).toBe("string");
      expect(
        mixnode.mixnode_details.bond_information.mix_node.http_api_port
      ).toStrictEqual(8000);
      expect(
        typeof mixnode.mixnode_details.bond_information.mix_node.verloc_port
      ).toBe("number");
      expect(
        typeof mixnode.mixnode_details.bond_information.mix_node.mix_port
      ).toBe("number");
      expect(
        mixnode.mixnode_details.bond_information.mix_node.mix_port
      ).toStrictEqual(1789);
      expect(
        typeof mixnode.mixnode_details.bond_information.mix_node.verloc_port
      ).toBe("number");

      const identitykey2 =
        mixnode.mixnode_details.bond_information.mix_node.identity_key;
      if (typeof identitykey2 === "string") {
        if (identitykey2.length === 43) {
          return true;
        } else expect(identitykey2).toHaveLength(44);
      }

      const sphinx2 =
        mixnode.mixnode_details.bond_information.mix_node.sphinx_key;
      if (typeof sphinx2 === "string") {
        if (sphinx2.length === 43) {
          return true;
        } else expect(sphinx2).toHaveLength(44);
      }

      //mixnode rewarding info
      expect(
        typeof mixnode.mixnode_details.rewarding_details.cost_params
          .profit_margin_percent
      ).toBe("string");
      expect(
        typeof mixnode.mixnode_details.rewarding_details.cost_params
          .interval_operating_cost.denom
      ).toBe("string");
      expect(
        typeof mixnode.mixnode_details.rewarding_details.cost_params
          .interval_operating_cost.amount
      ).toBe("string");
      expect(typeof mixnode.mixnode_details.rewarding_details.operator).toBe(
        "string"
      );
      expect(typeof mixnode.mixnode_details.rewarding_details.delegates).toBe(
        "string"
      );
      expect(
        typeof mixnode.mixnode_details.rewarding_details.total_unit_reward
      ).toBe("string");
      expect(
        typeof mixnode.mixnode_details.rewarding_details.unit_delegation
      ).toBe("string");
      expect(
        typeof mixnode.mixnode_details.rewarding_details.last_rewarded_epoch
      ).toBe("number");
      expect(
        typeof mixnode.mixnode_details.rewarding_details.unique_delegations
      ).toBe("number");
    });
  });

  it("Get active mixnodes", async (): Promise<void> => {
    const response = await contract.getActiveMixnodes();
    response.forEach(function (mixnode) {
      expect(
        mixnode.rewarding_details.cost_params.profit_margin_percent
      ).toBeTruthy();
      expect(typeof mixnode.bond_information.layer).toBe("number");
    });
  });

  it("Get active mixnodes detailed", async (): Promise<void> => {
    const response = await contract.getActiveMixnodesDetailed();
    response.forEach(function (mixnode) {
      expect(
        mixnode.mixnode_details.rewarding_details.cost_params
          .profit_margin_percent
      ).toBeTruthy();
    });
  });

  it("Get rewarded mixnodes", async (): Promise<void> => {
    const response = await contract.getRewardedMixnodes();
    response.forEach(function (mixnode) {
      expect(mixnode.rewarding_details.last_rewarded_epoch).toBeTruthy();
    });
  });

  it("Get rewarded mixnodes detailed", async (): Promise<void> => {
    const response = await contract.getRewardedMixnodesDetailed();
    response.forEach(function (mixnode) {
      expect(
        mixnode.mixnode_details.rewarding_details.last_rewarded_epoch
      ).toBeTruthy();
    });
  });

  it("Get blacklisted mixnodes", async (): Promise<void> => {
    const response = await contract.getBlacklistedMixnodes();
    if (response === null) {
      // no blacklisted mixnodes returns an empty array
      expect(response).toBeNull();
    } else {
      response.forEach(function (value) {
        expect(typeof value).toBe("number");
      });
    }
  });
});
