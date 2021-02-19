import { Test } from 'mocha';
import { assert } from 'chai';
import INetClient from '../net-client';
import { Fixtures } from '../../tests/fixtures'
import { Mock } from 'moq.ts';
import { MixnodesCache } from './mixnodes'

describe("Retrieving mixnodes: when the validator returns", () => {
    context("an empty list", () => {
        it("Should hold an empty list", () => {
            const perPage = 100;
            const mockClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, perPage)).returns([]);
            const chainCache = new MixnodesCache(mockClient.object(), perPage);

            chainCache.refreshMixNodes();

            let result = chainCache.mixNodes;
            assert.deepEqual([], result);
        });
    })
    context("a list of nodes that fits in a page", () => {
        it("Should return the list", () => {
            const perPage = 100;
            const mockClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, perPage)).returns(Fixtures.nodeList2());
            const cache = new MixnodesCache(mockClient.object(), perPage);

            cache.refreshMixNodes();

            let result = cache.mixNodes;
            assert.deepEqual(Fixtures.nodeList2(), result);
        })
    })

    context("a list of nodes that is longer than one page", () => {
        it("Should return the list", () => {
            const perPage = 100;
            const mockClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, perPage)).returns(Fixtures.nodeList2());
            const cache = new MixnodesCache(mockClient.object(), perPage);

            cache.refreshMixNodes();

            let result = cache.mixNodes;
            assert.deepEqual(Fixtures.nodeList2(), result);
        })
    })
});
