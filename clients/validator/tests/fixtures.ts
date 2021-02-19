import { MixNode } from '../src/types'

export class Fixtures {

    static Mixnode(): MixNode {
        return {
            stake: 1,
            layer: 1,
            pubKey: "foo"
        };
    }
}

