import axios from "axios";
import ConfigHandler from "../../../../common/api-test-utils/config/configHandler"

// Get the current env from configHandler
const configHandler = ConfigHandler.getInstance();
const currentEnvironment = process.env.TEST_ENV || "sandbox" || "prod";
const apiBaseUrl =
  configHandler.getEnvironmentConfig(currentEnvironment).api_base_url;

// get the gateway ip addresses
export async function getGatewayIPAddresses(): Promise<string[]> {
  try {
    const response = await axios.get(`${apiBaseUrl}/gateways`);
    if (response.status === 200) {
      const hosts = response.data.map((item: { gateway: { host: string } }) => {
        const host = item.gateway.host;
        const apiUrl = `http://${host}:8080/api/v1`;
        // console.log(`API URL for host ${host}: ${apiUrl}`);
        return apiUrl;
      });
      return hosts;
    } else {
      throw new Error("Failed to fetch gateway hosts.");
    }
  } catch (error) {
    throw new Error(`Error fetching gateway IP addresses: ${error.message}`);
  }
}
