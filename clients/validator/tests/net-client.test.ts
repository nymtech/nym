import 'mocha';
import { assert } from 'chai';
import { expect } from 'chai';
import NetClient from '../src/net-client';
// import MixNode from '../src/types';
import { Test } from 'mocha';
import * as rest from 'typed-rest-client/RestClient'
import { Fixtures } from './fixtures'


interface MixNode {
    stake: number,
    pubKey: string,
    layer: number,
}

describe("Validator network client", () => {
    describe("Retrieving mixnodes, when the validator returns", () => {
        context("an empty list", () => {
            it("Should return an empty list", () => {
                let client = new NetClient();

                var result: MixNode[] = client.GetMixnodes(1, 2);
                assert.deepEqual([], result);
            });
        })
    })
});
