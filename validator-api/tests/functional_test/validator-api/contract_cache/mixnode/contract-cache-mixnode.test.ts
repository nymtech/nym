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
      expect(typeof mixnode.accumulated_rewards).toStrictEqual('string');
       //expect(typeof mixnode.proxy).toStrictEqual('string'); could be null could be string
      
      //pledge 
      expect(typeof mixnode.pledge_amount.amount).toStrictEqual('string');
      expect(mixnode.pledge_amount.denom).toStrictEqual('unym');

      //total_deleglation
      expect(typeof mixnode.total_delegation.amount).toStrictEqual('string')
      expect(mixnode.total_delegation.denom).toStrictEqual('unym');

      //mixnode
      expect(typeof mixnode.mix_node.host).toStrictEqual('string')
      expect(typeof mixnode.mix_node.profit_margin_percent).toStrictEqual('number');
      expect(typeof mixnode.mix_node.identity_key).toStrictEqual('string'); //identity keys are 43 || 44 characters in length - check range
      expect(typeof mixnode.mix_node.sphinx_key).toStrictEqual('string'); //sphinx keys are 43 || 44 characters in length - check range
      expect(mixnode.mix_node.verloc_port).toStrictEqual(1790);
      expect(mixnode.mix_node.mix_port).toStrictEqual(1789);
      expect(mixnode.mix_node.http_api_port).toStrictEqual(8000);
      expect(typeof mixnode.mix_node.version).toStrictEqual('string');
     
    });
  });
});
