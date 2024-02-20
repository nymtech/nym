import Gateway from "../../src/endpoints/Gateways";
import { getGatewayIPAddresses } from "../../src/helpers/helper";

describe("Get gateway related info", (): void => {
    let contract: Gateway;
    let gatewayHosts: string[];
    beforeAll(async (): Promise<void> => {
        try {
            gatewayHosts = await getGatewayIPAddresses();
            console.log(gatewayHosts);
        } catch (error) {
            throw new Error(`Error fetching gateway IP addresses: ${error.message}`);
        }
    });

    beforeEach(async (): Promise<void> => {
        for (let i = 0; i < gatewayHosts.length; i++) {
            // console.log("currently trying gateway host", gatewayHosts[i]);
            contract = new Gateway(gatewayHosts[i]);
        }
    });

    // TODO this test is failing, it's incorrectly entering the else statement
    it("Get root gateway information", async (): Promise<void> => {
        const response = await contract.getGatewayInformation();
        console.log(response)
        console.log(response.wireguard)
        if (response.wireguard === null) {
            console.log("This is the code I should be entering............")
            expect(response.wireguard).toBeNull();
            expect(typeof response.mixnet_websockets.ws_port).toBe("number");
            expect(typeof response.mixnet_websockets.wss_port).toBe("number");
            return;
        } else {
            console.log("This is wrong, I shouldn't be in the else statement.........")
            console.log(response.wireguard);
            expect(typeof response.wireguard.port).toBe("number");
            expect(typeof response.wireguard.public_key).toBe("string");
            expect(typeof response.mixnet_websockets.ws_port).toBe("number");
            expect(typeof response.mixnet_websockets.wss_port).toBe("number");
        }
    });

    it("Get client interfaces supported by gateway", async (): Promise<void> => {
        const response = await contract.getGatewayClientInterfaces();
        if (response.wireguard === null) {
            expect(response.wireguard).toBeNull();
        } else {
            expect(typeof response.wireguard.port).toBe("number");
            expect(typeof response.wireguard.public_key).toBe("string");
            expect(typeof response.mixnet_websockets.ws_port).toBe("number");
            expect(typeof response.mixnet_websockets.wss_port).toBe("number");
        }
    });

    it("Get mixnet websocket info", async (): Promise<void> => {
        const response = await contract.getMixnetWebsocketInfo();
        expect(typeof response.ws_port).toBe("number");
        expect(typeof response.wss_port === ("number") || response.wss_port === null).toBe(true);
    });

    //   it("Get wireguard info", async (): Promise<void> => {
    //     const response = await contract.getWireguardInfo();
    //     expect(typeof response.port).toBe("number");
    //     expect(typeof response.public_key).toBe("string");
    //   });
});
