import expect from 'expect';

export const delegation = {
    owner: expect.any(String),
    mix_id: expect.any(Number),
    cumulative_reward_ratio: expect.any(String),
    amount: {
        denom: expect.any(String),
        amount: expect.any(String)
    },
    height: expect.any(Number || BigInt),
    proxy: expect.any(String || null)
};

export const gateway = {
    pledge_amount: {
        denom: expect.any(String),
        amount: expect.any(String)
    },
    owner: expect.any(String),
    block_height: expect.any(Number || BigInt),
    gateway: {
        host: expect.any(String),
        mix_port: expect.any(Number),
        clients_port: expect.any(Number),
        location: expect.any(String),
        sphinx_key: expect.any(String),
        identity_key: expect.any(String),
        version: expect.any(String),
    },
    proxy: expect.any(String || null)
};

export const mixnode = {
    bond_information: {
        mix_id: expect.any(Number),
        owner: expect.any(String),
        original_pledge: {
            denom: expect.any(String),
            amount: expect.any(String)
        },
        layer: expect.any(String),
        mix_node: {
            host: expect.any(String),
            mix_port: expect.any(Number),
            verloc_port: expect.any(Number),
            http_api_port: expect.any(Number),
            sphinx_key: expect.any(String),
            identity_key: expect.any(String),
            version: expect.any(String)
        },
        proxy: expect.any(String) || null,
        bonding_height: expect.any(Number || BigInt),
        is_unbonding: expect.any(Boolean)
    },
    rewarding_details: {
        cost_params: {
            profit_margin_percent: expect.any(String),
            interval_operating_cost: {
                denom: expect.any(String),
                amount: expect.any(String)
            }
        },
        operator: expect.any(String),
        delegates: expect.any(String),
        total_unit_reward: expect.any(String),
        unit_delegation: expect.any(String),
        last_rewarded_epoch: expect.any(Number),
        unique_delegations: expect.any(Number)
    }
}

export const saturation = {
    saturation: expect.any(String),
    uncapped_saturation: expect.any(String),
    as_at: expect.any(Number || BigInt)
}

export const contract = {
    minimum_mixnode_pledge: expect.any(String),
    minimum_gateway_pledge: expect.any(String),
    mixnode_rewarded_set_size: expect.any(Number),
    mixnode_active_set_size: expect.any(Number)
}

export const mixnodebond = {
    mix_id: expect.any(Number),
    owner: expect.any(String),
    original_pledge: {
        denom: expect.any(String),
        amount: expect.any(String)
    },
    layer: expect.any(String),
    mix_node: {
        host: expect.any(String),
        mix_port: expect.any(Number),
        verloc_port: expect.any(Number),
        http_api_port: expect.any(Number),
        sphinx_key: expect.any(String),
        identity_key: expect.any(String),
        version: expect.any(String)
    },
    proxy: expect.any(String) || null,
    bonding_height: expect.any(Number || BigInt),
    is_unbonding: expect.any(Boolean)
}

export const unbondednode = {
    identity_key: expect.any(String),
    owner: expect.any(String),
    proxy: expect.any(String) || null,
    unbonding_height: expect.any(Number)
}