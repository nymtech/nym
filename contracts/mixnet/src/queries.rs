// settings for pagination
use crate::state::MixNodeBond;
use crate::state::PREFIX_MIXNODES;
use cosmwasm_std::Deps;
use cosmwasm_std::HumanAddr;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cosmwasm_storage::bucket_read;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_mixnodes_paged(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<MixNodeBond>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_ref().map(|addr| addr.as_bytes());

    let bucket = bucket_read::<MixNodeBond>(deps.storage, PREFIX_MIXNODES);
    let res = bucket.range(start, None, Order::Ascending).take(limit);
    let node_tuples = res.collect::<StdResult<Vec<(Vec<u8>, MixNodeBond)>>>()?;
    let nodes = node_tuples.into_iter().map(|item| item.1).collect();
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::query;
    use crate::msg::QueryMsg;
    use crate::state::mixnodes;
    use crate::support::tests::helpers;
    use cosmwasm_std::coins;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::mock_env;

    #[test]
    fn query_mixnodes_one_page() {
        let mut deps = helpers::init_contract();

        let result = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let nodes: Vec<MixNodeBond> = from_binary(&result).unwrap();
        assert_eq!(0, nodes.len());

        // let's add a node
        let node = MixNodeBond {
            amount: coins(50, "unym"),
            owner: HumanAddr::from("foo"),
            mix_node: helpers::mix_node_fixture(),
        };
        mixnodes(&mut deps.storage)
            .save("foo".as_bytes(), &node)
            .unwrap();

        // is the node there?
        let result = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let nodes: Vec<MixNodeBond> = from_binary(&result).unwrap();
        assert_eq!(1, nodes.len());
        assert_eq!(helpers::mix_node_fixture(), nodes[0].mix_node);
    }

    #[test]
    fn query_mixnodes_two_pages() {
        let mut deps = helpers::init_contract();

        let result = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let nodes: Vec<MixNodeBond> = from_binary(&result).unwrap();
        assert_eq!(0, nodes.len());

        // let's add a node
        let node = MixNodeBond {
            amount: coins(50, "unym"),
            owner: HumanAddr::from("foo"),
            mix_node: helpers::mix_node_fixture(),
        };
        mixnodes(&mut deps.storage)
            .save("foo".as_bytes(), &node)
            .unwrap();

        // is the node there?
        let result = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let nodes: Vec<MixNodeBond> = from_binary(&result).unwrap();
        assert_eq!(1, nodes.len());
        assert_eq!(helpers::mix_node_fixture(), nodes[0].mix_node);
    }

    #[test]
    fn mixnodes_empty_on_init() {
        let deps = helpers::init_contract();
        let all_nodes = query_mixnodes_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, all_nodes.len());
    }

    #[test]
    fn mixnodes_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_default_limit_10() {}

    #[test]
    fn mixnodes_paged_retrieval_has_max_limit_30() {}
}
