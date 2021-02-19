import { MixNode } from '../src/types'

export class Fixtures {

    static mixnode(): MixNode {
        return {
            stake: 1,
            layer: 1,
            pubKey: "foo"
        };
    }

    static nodeList2(): MixNode[] {
        return [Fixtures.mixnode(), Fixtures.mixnode()]
    }
}

