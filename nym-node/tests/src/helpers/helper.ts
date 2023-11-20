import { APIClient } from "../../src/endpoints/abstracts/APIClient";
import ConfigHandler from "../../../../common/api-test-utils/config/configHandler";

const configHandler = ConfigHandler.getInstance();

let helper: Helper;
const apiBaseUrl = configHandler.environmentConfig.api_base_url;
export default class Helper extends APIClient {
  constructor() {
    super(apiBaseUrl, "");
  }
}

export async function getGatewayIPAddresses(): Promise<string[]> {
  helper = new Helper();
  try {
    const response = await helper.restClient.sendGet({
      route: `/gateways`,
    });
    const hosts = response.map((item: { gateway: { host: string } }) => {
      const host = item.gateway.host;
      const apiUrl = `http://${host}:8080/api/v1`;
      return apiUrl;
    });
    return hosts;
  } catch (error) {
    throw new Error(`Error fetching gateway IP addresses: ${error.message}`);
  }
}

export async function getMixnodeIPAddresses(): Promise<string[]> {
  helper = new Helper();
  try {
    const response = await helper.restClient.sendGet({
      route: `/mixnodes`,
    });
    const hosts = response.map((item: { bond_information: { mix_node: { host: string } } } ) => {
      const host = item.bond_information.mix_node.host;
      const apiUrl = `http://${host}:8000/api/v1`;
      return apiUrl;
    });
    return hosts;
  } catch (error) {
    throw new Error(`Error fetching mixnode IP addresses: ${error.message}`);
  }
}
