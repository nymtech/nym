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
    let start = start_after.as_ref().map(|addr| {
        let mut thing = addr.as_bytes().to_owned();
        thing.push(0);
        thing
    });

    let bucket = bucket_read::<MixNodeBond>(deps.storage, PREFIX_MIXNODES);
    let res = bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit);
    let node_tuples = res.collect::<StdResult<Vec<(Vec<u8>, MixNodeBond)>>>()?;
    let nodes = node_tuples.into_iter().map(|item| item.1).collect();
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mixnodes;
    use crate::support::tests::helpers;

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
    fn mixnodes_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0..100 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        // query without explicitly setting a limit
        let page1 = query_mixnodes_paged(deps.as_ref(), None, None).unwrap();

        let expected_limit = 10;
        assert_eq!(expected_limit, page1.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = 30;
        assert_eq!(expected_limit, page1.len() as u32);
    }

    #[test]
    fn pagination_works() {
        let mut deps = helpers::init_contract();
        let node = helpers::mixnode_bond_fixture();
        mixnodes(&mut deps.storage)
            .save("1".as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.len());

        // save another
        mixnodes(&mut deps.storage)
            .save("2".as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.len());

        mixnodes(&mut deps.storage)
            .save("3".as_bytes(), &node)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.len());

        // retrieving the next page should start after the last key on this page
        let start_after = HumanAddr::from("2");
        let page2 = query_mixnodes_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.len());

        // save another one
        mixnodes(&mut deps.storage)
            .save("4".as_bytes(), &node)
            .unwrap();

        let start_after = HumanAddr::from("2");
        let page2 = query_mixnodes_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(2, page2.len());
    }
}
