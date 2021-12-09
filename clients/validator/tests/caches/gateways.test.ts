import { assert } from "chai";
import INetClient from "../../src/signing-client";
import { Fixtures } from "../fixtures"
import { Mock, Times } from "moq.ts";
import GatewaysCache from "../../src/caches/gateways";

describe("Caching gateways: when the validator returns", () => {
    context("an empty list", () => {
        it("Should return an empty list", async () => {
            const perPage = 100;
            const contractAddress = "mockContractAddress";
            const emptyPromise = Promise.resolve(Fixtures.GatewaysResp.empty());
            const mockClient = new Mock<INetClient>().setup(netClient => netClient.getGateways(contractAddress, perPage, undefined)).returns(emptyPromise);
            const cache = new GatewaysCache(mockClient.object(), perPage);

            await cache.refreshGateways(contractAddress);

            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, undefined), Times.Exactly(1));
            assert.deepEqual([], cache.gateways);
        });
    })

    context("a list of gatways that fits in a page", () => {
        it("Should return that one page list", async () => {
            const perPage = 2;
            const contractAddress = "mockContractAddress";
            const onePagePromise = Promise.resolve(Fixtures.GatewaysResp.onePage());
            const mockClient = new Mock<INetClient>().setup(netClient => netClient.getGateways(contractAddress, perPage, undefined)).returns(onePagePromise);
            const cache = new GatewaysCache(mockClient.object(), perPage);

            await cache.refreshGateways(contractAddress);

            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, undefined), Times.Exactly(1));
            assert.deepEqual(Fixtures.Gateways.list2(), cache.gateways);
        })
    })

    context("a list of gateways that is longer than one page", () => {
        it("Should return the full list assembled from all pages", async () => {
            const perPage = 2; // we get back 2 per page
            const contractAddress = "mockContractAddress";
            const fullPageResult = Fixtures.GatewaysResp.page1of2();
            const halfPageResult = Fixtures.GatewaysResp.halfPage2of2();
            const mockClient = new Mock<INetClient>()
            mockClient.setup(instance => instance.getGateways(contractAddress, perPage, undefined)).returns(Promise.resolve(fullPageResult));
            mockClient.setup(instance => instance.getGateways(contractAddress, perPage, fullPageResult.start_next_after)).returns(Promise.resolve(halfPageResult));
            const cache = new GatewaysCache(mockClient.object(), perPage);

            await cache.refreshGateways(contractAddress); // should make multiple paginated requests because there are two pages in the response fixture
            mockClient.verify(instance => instance.getGateways(contractAddress, perPage, undefined), Times.Exactly(1));
            mockClient.verify(instance => instance.getGateways(contractAddress, perPage, fullPageResult.start_next_after), Times.Exactly(1));

            assert.deepEqual(Fixtures.Gateways.list3(), cache.gateways); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })

    context("a list of gateways that is two filled pages", () => {
        it("Should return the full list assembled from all pages", async () => {
            const perPage = 2; // we get back 2 per page
            const contractAddress = "mockContractAddress";
            const fullPageResult1 = Fixtures.GatewaysResp.page1of2();
            const fullPageResult2 = Fixtures.GatewaysResp.fullPage2of2();
            const mockClient = new Mock<INetClient>()
            mockClient.setup(netClient => netClient.getGateways(contractAddress, perPage, undefined)).returns(Promise.resolve(fullPageResult1));
            mockClient.setup(netClient => netClient.getGateways(contractAddress, perPage, fullPageResult1.start_next_after)).returns(Promise.resolve(fullPageResult2));

            const cache = new GatewaysCache(mockClient.object(), perPage);

            await cache.refreshGateways(contractAddress); // should make multiple paginated requests because there are two pages in the response fixture
            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, undefined), Times.Exactly(1));
            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, fullPageResult1.start_next_after), Times.Exactly(1));

            assert.deepEqual(Fixtures.Gateways.list4(), cache.gateways); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })

    context("refreshing the cache twice", () => {
        it("returns one full list assembled from all pages", async () => {
            const perPage = 2; // we get back 2 per page
            const contractAddress = "mockContractAddress";
            const fullPageResult1 = Fixtures.GatewaysResp.page1of2();
            const fullPageResult2 = Fixtures.GatewaysResp.fullPage2of2();
            const mockClient = new Mock<INetClient>()
            mockClient.setup(netClient => netClient.getGateways(contractAddress, perPage, undefined)).returns(Promise.resolve(fullPageResult1));
            mockClient.setup(netClient => netClient.getGateways(contractAddress, perPage, fullPageResult1.start_next_after)).returns(Promise.resolve(fullPageResult2));

            const cache = new GatewaysCache(mockClient.object(), perPage);

            await cache.refreshGateways(contractAddress); // should make multiple paginated requests because there are two pages in the response fixture
            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, undefined), Times.Exactly(1));
            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, fullPageResult1.start_next_after), Times.Exactly(1));

            await cache.refreshGateways(contractAddress);
            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, undefined), Times.Exactly(2));
            mockClient.verify(netClient => netClient.getGateways(contractAddress, perPage, fullPageResult1.start_next_after), Times.Exactly(2));

            assert.deepEqual(Fixtures.Gateways.list4(), cache.gateways); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })
});
