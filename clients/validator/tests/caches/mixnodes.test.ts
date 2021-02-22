import { assert } from 'chai';
import INetClient from '../../src/net-client';
import { Fixtures } from '../fixtures'
import { Mock, Times } from 'moq.ts';
import { MixnodesCache } from '../../src/caches/mixnodes'

describe("Caching mixnodes: when the validator returns", () => {
    context("an empty list", () => {
        it("Should hold an empty list", async () => {
            const perPage = 100;
            const mockResponse = Fixtures.MixNodesResp.empty();
            const mockPromise = Promise.resolve(mockResponse);
            const mockClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, perPage)).returns(mockPromise);
            const mixnodesCache = new MixnodesCache(mockClient.object(), perPage);

            await mixnodesCache.refreshMixNodes();

            assert.deepEqual([], mixnodesCache.mixNodes);
        });
    })
    context("a list of nodes that fits in a page", () => {
        it("Should return that one page list", async () => {
            const perPage = 2;
            const onePageResult = Promise.resolve(Fixtures.MixNodesResp.onePage());
            const mockClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, perPage)).returns(onePageResult);
            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes();

            mockClient.verify(instance => instance.getMixnodes(1, perPage), Times.Exactly(1));
            assert.deepEqual(Fixtures.MixNodes.list2(), cache.mixNodes);
        })
    })

    context("a list of nodes that is longer than one page", () => {
        it("Should return the full list", async () => {
            const perPage = 2; // we get back 2 per page
            const partialResult = Promise.resolve(Fixtures.MixNodesResp.partial());
            const mockClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, perPage)).returns(partialResult);
            const cache = new MixnodesCache(mockClient.object(), perPage);

            await cache.refreshMixNodes(); // makes multiple paginated requests
            mockClient.verify(instance => instance.getMixnodes(1, 2), Times.Exactly(1));
            mockClient.verify(instance => instance.getMixnodes(2, 2), Times.Exactly(1));

            assert.deepEqual(Fixtures.MixNodes.list3(), cache.mixNodes); // there are a total of 3 nodes in the validator lists, we get them all back
        })
    })
});
