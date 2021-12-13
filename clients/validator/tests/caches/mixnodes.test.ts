import { assert } from "chai";
import INetClient from "../../src/signing-client";
import { Fixtures } from "../fixtures"
import { Mock, Times } from "moq.ts";
import { MixnodesCache } from "../../src/caches/mixnodes"

describe("Caching mixnodes: when the validator returns", () => {
    context("an empty list", () => {
        it("Should return an empty list", async () => {
            const perPage = 100;
            const contractAddress = "mockContractAddress";
            const emptyPromise = Promise.resolve(Fixtures.MixNodesResp.empty());
            const mockClient = new Mock<INetClient>().setup(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined)).returns(emptyPromise);
            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes(contractAddress);

            mockClient.verify(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined), Times.Exactly(1));
            assert.deepEqual([], cache.mixNodes);
        });
    })
    context("a list of nodes that fits in a page", () => {
        it("Should return that one page list", async () => {
            const perPage = 2;
            const contractAddress = "mockContractAddress";
            const onePagePromise = Promise.resolve(Fixtures.MixNodesResp.onePage());
            const mockClient = new Mock<INetClient>().setup(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined)).returns(onePagePromise);
            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes(contractAddress);

            mockClient.verify(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined), Times.Exactly(1));
            assert.deepEqual(Fixtures.MixNodes.list2(), cache.mixNodes);
        })
    })

    context("a list of nodes that is longer than one page", () => {
        it("Should return the full list assembled from all pages", async () => {
            const perPage = 2; // we get back 2 per page
            const contractAddress = "mockContractAddress";
            const fullPageResult = Fixtures.MixNodesResp.page1of2();
            const halfPageResult = Fixtures.MixNodesResp.halfPage2of2();
            const mockClient = new Mock<INetClient>()
            mockClient.setup(instance => instance.getMixNodesPaged(contractAddress, perPage, undefined)).returns(Promise.resolve(fullPageResult));
            mockClient.setup(instance => instance.getMixNodesPaged(contractAddress, perPage, fullPageResult.start_next_after)).returns(Promise.resolve(halfPageResult));
            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes(contractAddress); // should make multiple paginated requests because there are two pages in the response fixture
            mockClient.verify(instance => instance.getMixNodesPaged(contractAddress, perPage, undefined), Times.Exactly(1));
            mockClient.verify(instance => instance.getMixNodesPaged(contractAddress, perPage, fullPageResult.start_next_after), Times.Exactly(1));

            assert.deepEqual(Fixtures.MixNodes.list3(), cache.mixNodes); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })

    context("a list of nodes that is two filled pages", () => {
        it("Should return the full list assembled from all pages", async () => {
            const perPage = 2; // we get back 2 per page
            const contractAddress = "mockContractAddress";
            const fullPageResult1 = Fixtures.MixNodesResp.page1of2();
            const fullPageResult2 = Fixtures.MixNodesResp.fullPage2of2();
            const mockClient = new Mock<INetClient>()
            mockClient.setup(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined)).returns(Promise.resolve(fullPageResult1));
            mockClient.setup(netClient => netClient.getMixNodesPaged(contractAddress, perPage, fullPageResult1.start_next_after)).returns(Promise.resolve(fullPageResult2));

            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes(contractAddress); // should make multiple paginated requests because there are two pages in the response fixture
            mockClient.verify(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined), Times.Exactly(1));
            mockClient.verify(netClient => netClient.getMixNodesPaged(contractAddress, perPage, fullPageResult1.start_next_after), Times.Exactly(1));

            assert.deepEqual(Fixtures.MixNodes.list4(), cache.mixNodes); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })

    context("refreshing the cache twice", () => {
        it("returns one full list assembled from all pages", async () => {
            const perPage = 2; // we get back 2 per page
            const contractAddress = "mockContractAddress";
            const fullPageResult1 = Fixtures.MixNodesResp.page1of2();
            const fullPageResult2 = Fixtures.MixNodesResp.fullPage2of2();
            const mockClient = new Mock<INetClient>()
            mockClient.setup(netClient => netClient.getMixNodesPaged(contractAddress, perPage, undefined)).returns(Promise.resolve(fullPageResult1));
            mockClient.setup(netClient => netClient.getMixNodesPaged(contractAddress, perPage, fullPageResult1.start_next_after)).returns(Promise.resolve(fullPageResult2));

            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes(contractAddress); // should make multiple paginated requests because there are two pages in the response fixture
            await cache.refreshMixNodes(contractAddress);
            // mockClient.verify(netClient => netClient.getMixNodes(contractAddress, perPage, undefined), Times.Exactly(1));
            // mockClient.verify(netClient => netClient.getMixNodes(contractAddress, perPage, fullPageResult1.start_next_after), Times.Exactly(1));

            assert.deepEqual(Fixtures.MixNodes.list4(), cache.mixNodes); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })
});
