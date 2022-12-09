import type { DecCoin } from './DecCoin';
export declare type PendingEpochEventData = {
    Delegate: {
        owner: string;
        mix_id: number;
        amount: DecCoin;
        proxy: string | null;
    };
} | {
    Undelegate: {
        owner: string;
        mix_id: number;
        proxy: string | null;
    };
} | {
    UnbondMixnode: {
        mix_id: number;
    };
} | {
    UpdateActiveSetSize: {
        new_size: number;
    };
};
//# sourceMappingURL=PendingEpochEventData.d.ts.map