import { Test } from 'mocha';
import { assert } from 'chai';
import INetClient from '../src/net-client';
import { Fixtures } from './fixtures'
import { Mock } from 'moq.ts';
import { ChainCache } from '../src/chaincache'

describe("Retrieving mixnodes, when the validator returns", () => {
    context("an empty list", () => {
        it("Should hold an empty list", () => {
            const mockClient: INetClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, 100)).returns([]);
            const chainCache = new ChainCache(mockClient.object());

            chainCache.refreshMixNodes();

            let result = chainCache.mixNodes;
            assert.deepEqual([], result);
        });
    })
    context("a populated list", () => {
        it("Should return the list", () => {
            const mockClient: INetClient = new Mock<INetClient>().setup(instance => instance.getMixnodes(1, 100)).returns(Fixtures.nodeList2());
            const chainCache = new ChainCache(mockClient.object());

            chainCache.refreshMixNodes();

            let result = chainCache.mixNodes;
            assert.deepEqual(Fixtures.nodeList2(), result);
        })
    })
});
