use std::cmp::Ordering;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, BlockInfo, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order,
    Response, StdResult,
};

use cw2::set_contract_version;

use cw3::{
    Ballot, Proposal, ProposalListResponse, ProposalResponse, Status, Vote, VoteInfo,
    VoteListResponse, VoteResponse, VoterDetail, VoterListResponse, VoterResponse, Votes,
};
use cw3_fixed_multisig::state::{next_id, BALLOTS, PROPOSALS};
use cw4::{Cw4Contract, MemberChangedHookMsg, MemberDiff};
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, Expiration, ThresholdResponse};

use nym_multisig_contract_common::error::ContractError;
use nym_multisig_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_multisig_contract_common::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw3-flex-multisig";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let group_addr = Cw4Contract(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        ContractError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);
    // Those might need to be changed via a migration, due to circular dependency
    // of deploying the two contracts
    let coconut_bandwidth_addr = deps
        .api
        .addr_validate(&msg.coconut_bandwidth_contract_address)?;
    let coconut_dkg_addr = deps.api.addr_validate(&msg.coconut_dkg_contract_address)?;
    let total_weight = group_addr.total_weight(&deps.querier)?;
    msg.threshold.validate(total_weight)?;

    let proposal_deposit = msg
        .proposal_deposit
        .map(|deposit| deposit.into_checked(deps.as_ref()))
        .transpose()?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        threshold: msg.threshold,
        max_voting_period: msg.max_voting_period,
        group_addr,
        coconut_bandwidth_addr,
        coconut_dkg_addr,
        executor: msg.executor,
        proposal_deposit,
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest,
        } => execute_propose(deps, env, info, title, description, msgs, latest),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
        ExecuteMsg::MemberChangedHook(MemberChangedHookMsg { diffs }) => {
            execute_membership_hook(deps, env, info, diffs)
        }
    }
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg>,
    // we ignore earliest
    latest: Option<Expiration>,
) -> Result<Response<Empty>, ContractError> {
    // only members of the multisig can create a proposal
    let cfg = CONFIG.load(deps.storage)?;

    // Check that the native deposit was paid (as needed).
    if let Some(deposit) = cfg.proposal_deposit.as_ref() {
        deposit.check_native_deposit_paid(&info)?;
    }

    // Only the coconut bandwidth or dkg contracts can create proposals
    if info.sender != cfg.coconut_bandwidth_addr && info.sender != cfg.coconut_dkg_addr {
        return Err(ContractError::Unauthorized {});
    }
    // The contract doesn't have any say in the voting outcome
    let vote_power = 0;

    // max expires also used as default
    let max_expires = cfg.max_voting_period.after(&env.block);
    let mut expires = latest.unwrap_or(max_expires);
    let comp = expires.partial_cmp(&max_expires);
    if let Some(Ordering::Greater) = comp {
        expires = max_expires;
    } else if comp.is_none() {
        return Err(ContractError::WrongExpiration {});
    }

    // Take the cw20 token deposit, if required. We do this before
    // creating the proposal struct below so that we can avoid a clone
    // and move the loaded deposit info into it.
    let take_deposit_msg = if let Some(deposit_info) = cfg.proposal_deposit.as_ref() {
        deposit_info.get_take_deposit_messages(&info.sender, &env.contract.address)?
    } else {
        vec![]
    };

    // create a proposal
    let mut prop = Proposal {
        title,
        description,
        start_height: env.block.height,
        expires,
        msgs,
        status: Status::Open,
        votes: Votes::yes(vote_power),
        threshold: cfg.threshold,
        total_weight: cfg.group_addr.total_weight(&deps.querier)?,
        proposer: info.sender.clone(),
        deposit: cfg.proposal_deposit,
    };
    prop.update_status(&env.block);
    let id = next_id(deps.storage)?;
    PROPOSALS.save(deps.storage, id, &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight: vote_power,
        vote: Vote::Yes,
    };
    BALLOTS.save(deps.storage, (id, &info.sender), &ballot)?;

    Ok(Response::new()
        .add_messages(take_deposit_msg)
        .add_attribute("action", "propose")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: Vote,
) -> Result<Response<Empty>, ContractError> {
    // only members of the multisig can vote
    let cfg = CONFIG.load(deps.storage)?;

    // ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    // Allow voting on Passed and Rejected proposals too,
    if ![Status::Open, Status::Passed, Status::Rejected].contains(&prop.status) {
        return Err(ContractError::NotOpen {});
    }
    // if they are not expired
    if prop.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // Only voting members of the multisig can vote
    // Additional check if weight >= 1
    // use a snapshot of "start of proposal"
    let vote_power = cfg
        .group_addr
        .is_voting_member(&deps.querier, &info.sender, prop.start_height)?
        .ok_or(ContractError::Unauthorized {})?;

    // cast vote if no vote previously cast
    BALLOTS.update(deps.storage, (proposal_id, &info.sender), |bal| match bal {
        Some(_) => Err(ContractError::AlreadyVoted {}),
        None => Ok(Ballot {
            weight: vote_power,
            vote,
        }),
    })?;

    // update vote tally
    prop.votes.add_vote(vote, vote_power);
    prop.update_status(&env.block);
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::new()
        .add_attribute("action", "vote")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    // we allow execution even after the proposal "expiration" as long as all vote come in before
    // that point. If it was approved on time, it can be executed any time.
    prop.update_status(&env.block);
    if prop.status != Status::Passed {
        return Err(ContractError::WrongExecuteStatus {});
    }

    let cfg = CONFIG.load(deps.storage)?;
    cfg.authorize(&deps.querier, &info.sender)?;

    // set it to executed
    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    // Unconditionally refund here.
    let response = match prop.deposit {
        Some(deposit) => {
            Response::new().add_message(deposit.get_return_deposit_message(&prop.proposer)?)
        }
        None => Response::new(),
    };

    // dispatch all proposed messages
    Ok(response
        .add_messages(prop.msgs)
        .add_attribute("action", "execute")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response<Empty>, ContractError> {
    // anyone can trigger this if the vote passed

    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    if [Status::Executed, Status::Rejected, Status::Passed].contains(&prop.status) {
        return Err(ContractError::WrongCloseStatus {});
    }
    // Avoid closing of Passed due to expiration proposals
    if prop.current_status(&env.block) == Status::Passed {
        return Err(ContractError::WrongCloseStatus {});
    }
    if !prop.expires.is_expired(&env.block) {
        return Err(ContractError::NotExpired {});
    }

    // set it to failed
    prop.status = Status::Rejected;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    // Refund the deposit if we have been configured to do so.
    let mut response = Response::new();
    if let Some(deposit) = prop.deposit {
        if deposit.refund_failed_proposals {
            response = response.add_message(deposit.get_return_deposit_message(&prop.proposer)?)
        }
    }

    Ok(response
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_membership_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _diffs: Vec<MemberDiff>,
) -> Result<Response<Empty>, ContractError> {
    // This is now a no-op
    // But we leave the authorization check as a demo
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.group_addr.0 {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, env, proposal_id)?),
        QueryMsg::Vote { proposal_id, voter } => to_binary(&query_vote(deps, proposal_id, voter)?),
        QueryMsg::ListProposals { start_after, limit } => {
            to_binary(&list_proposals(deps, env, start_after, limit)?)
        }
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => to_binary(&reverse_proposals(deps, env, start_before, limit)?),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&list_votes(deps, proposal_id, start_after, limit)?),
        QueryMsg::Voter { address } => to_binary(&query_voter(deps, address)?),
        QueryMsg::ListVoters { start_after, limit } => {
            to_binary(&list_voters(deps, start_after, limit)?)
        }
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_threshold(deps: Deps) -> StdResult<ThresholdResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_weight = cfg.group_addr.total_weight(&deps.querier)?;
    Ok(cfg.threshold.to_response(total_weight))
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id)?;
    let status = prop.current_status(&env.block);
    let threshold = prop.threshold.to_response(prop.total_weight);
    Ok(ProposalResponse {
        id,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        status,
        expires: prop.expires,
        proposer: prop.proposer,
        deposit: prop.deposit,
        threshold,
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_proposals(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    let proposals = PROPOSALS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect::<StdResult<_>>()?;

    Ok(ProposalListResponse { proposals })
}

fn reverse_proposals(
    deps: Deps,
    env: Env,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_before.map(Bound::exclusive);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn map_proposal(
    block: &BlockInfo,
    item: StdResult<(u64, Proposal)>,
) -> StdResult<ProposalResponse> {
    item.map(|(id, prop)| {
        let status = prop.current_status(block);
        let threshold = prop.threshold.to_response(prop.total_weight);
        ProposalResponse {
            id,
            title: prop.title,
            description: prop.description,
            msgs: prop.msgs,
            status,
            expires: prop.expires,
            deposit: prop.deposit,
            proposer: prop.proposer,
            threshold,
        }
    })
}

fn query_vote(deps: Deps, proposal_id: u64, voter: String) -> StdResult<VoteResponse> {
    let voter_addr = deps.api.addr_validate(&voter)?;
    let prop = BALLOTS.may_load(deps.storage, (proposal_id, &voter_addr))?;
    let vote = prop.map(|b| VoteInfo {
        proposal_id,
        voter,
        vote: b.vote,
        weight: b.weight,
    });
    Ok(VoteResponse { vote })
}

fn list_votes(
    deps: Deps,
    proposal_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<VoteListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let votes = BALLOTS
        .prefix(proposal_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, ballot)| VoteInfo {
                proposal_id,
                voter: addr.into(),
                vote: ballot.vote,
                weight: ballot.weight,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(VoteListResponse { votes })
}

fn query_voter(deps: Deps, voter: String) -> StdResult<VoterResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voter_addr = deps.api.addr_validate(&voter)?;
    let weight = cfg.group_addr.is_member(&deps.querier, &voter_addr, None)?;

    Ok(VoterResponse { weight })
}

fn list_voters(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<VoterListResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voters = cfg
        .group_addr
        .list_members(&deps.querier, start_after, limit)?
        .into_iter()
        .map(|member| VoterDetail {
            addr: member.addr,
            weight: member.weight,
        })
        .collect();
    Ok(VoterListResponse { voters })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    cfg.coconut_bandwidth_addr = deps.api.addr_validate(&msg.coconut_bandwidth_address)?;
    cfg.coconut_dkg_addr = deps.api.addr_validate(&msg.coconut_dkg_address)?;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, coins, Addr, BankMsg, Coin, Decimal, Timestamp, Uint128};

    use cw2::{query_contract_info, ContractVersion};
    use cw20::{Cw20Coin, UncheckedDenom};
    use cw3::{DepositError, UncheckedDepositInfo};
    use cw4::{Cw4ExecuteMsg, Member};
    use cw4_group::helpers::Cw4GroupContract;
    use cw_multi_test::{
        next_block, App, AppBuilder, BankSudo, Contract, ContractWrapper, Executor, SudoMsg,
    };
    use cw_utils::{Duration, Threshold};

    use super::*;

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const VOTER4: &str = "voter0004";
    const VOTER5: &str = "voter0005";
    const SOMEBODY: &str = "somebody";
    const BANDWIDTH_CONTRACT: &str = "coconut_bandwidth_addr";
    const DKG_CONTRACT: &str = "coconut_dkg_addr";

    fn member<T: Into<String>>(addr: T, weight: u64) -> Member {
        Member {
            addr: addr.into(),
            weight,
        }
    }

    pub fn contract_flex() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_group() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw4_group::contract::execute,
            cw4_group::contract::instantiate,
            cw4_group::contract::query,
        );
        Box::new(contract)
    }

    fn contract_cw20() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app(init_funds: &[Coin]) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(OWNER), init_funds.to_vec())
                .unwrap();
        })
    }

    // uploads code and returns address of group contract
    fn instantiate_group(app: &mut App, members: Vec<Member>) -> Addr {
        let group_id = app.store_code(contract_group());
        let msg = nym_group_contract_common::msg::InstantiateMsg {
            admin: Some(OWNER.into()),
            members,
        };
        app.instantiate_contract(group_id, Addr::unchecked(OWNER), &msg, &[], "group", None)
            .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    #[track_caller]
    fn instantiate_flex(
        app: &mut App,
        group: Addr,
        coconut_bandwidth_contract_address: Addr,
        coconut_dkg_contract_address: Addr,
        threshold: Threshold,
        max_voting_period: Duration,
        executor: Option<nym_multisig_contract_common::state::Executor>,
        proposal_deposit: Option<UncheckedDepositInfo>,
    ) -> Addr {
        let flex_id = app.store_code(contract_flex());
        let msg = InstantiateMsg {
            group_addr: group.to_string(),
            coconut_bandwidth_contract_address: coconut_bandwidth_contract_address.to_string(),
            coconut_dkg_contract_address: coconut_dkg_contract_address.to_string(),
            threshold,
            max_voting_period,
            executor,
            proposal_deposit,
        };
        app.instantiate_contract(flex_id, Addr::unchecked(OWNER), &msg, &[], "flex", None)
            .unwrap()
    }

    // this will set up both contracts, instantiating the group with
    // all voters defined above, and the multisig pointing to it and given threshold criteria.
    // Returns (multisig address, group address).
    #[track_caller]
    fn setup_test_case_fixed(
        app: &mut App,
        weight_needed: u64,
        max_voting_period: Duration,
        init_funds: Vec<Coin>,
        multisig_as_group_admin: bool,
    ) -> (Addr, Addr) {
        setup_test_case(
            app,
            Threshold::AbsoluteCount {
                weight: weight_needed,
            },
            max_voting_period,
            init_funds,
            multisig_as_group_admin,
            None,
            None,
        )
    }

    #[track_caller]
    fn setup_test_case(
        app: &mut App,
        threshold: Threshold,
        max_voting_period: Duration,
        init_funds: Vec<Coin>,
        multisig_as_group_admin: bool,
        executor: Option<nym_multisig_contract_common::state::Executor>,
        proposal_deposit: Option<UncheckedDepositInfo>,
    ) -> (Addr, Addr) {
        // 1. Instantiate group contract with members (and OWNER as admin)
        let members = vec![
            member(OWNER, 0),
            member(VOTER1, 1),
            member(VOTER2, 2),
            member(VOTER3, 3),
            member(VOTER4, 12), // so that he alone can pass a 50 / 52% threshold proposal
            member(VOTER5, 5),
        ];
        let group_addr = instantiate_group(app, members);
        app.update_block(next_block);

        // 2. Set up Multisig backed by this group
        let flex_addr = instantiate_flex(
            app,
            group_addr.clone(),
            Addr::unchecked(BANDWIDTH_CONTRACT),
            Addr::unchecked(DKG_CONTRACT),
            threshold,
            max_voting_period,
            executor,
            proposal_deposit,
        );
        app.update_block(next_block);

        // 3. (Optional) Set the multisig as the group owner
        if multisig_as_group_admin {
            let update_admin = Cw4ExecuteMsg::UpdateAdmin {
                admin: Some(flex_addr.to_string()),
            };
            app.execute_contract(
                Addr::unchecked(OWNER),
                group_addr.clone(),
                &update_admin,
                &[],
            )
            .unwrap();
            app.update_block(next_block);
        }

        // Bonus: set some funds on the multisig contract for future proposals
        if !init_funds.is_empty() {
            app.send_tokens(Addr::unchecked(OWNER), flex_addr.clone(), &init_funds)
                .unwrap();
        }
        (flex_addr, group_addr)
    }

    fn proposal_info() -> (Vec<CosmosMsg<Empty>>, String, String) {
        let bank_msg = BankMsg::Send {
            to_address: SOMEBODY.into(),
            amount: coins(1, "BTC"),
        };
        let msgs = vec![bank_msg.into()];
        let title = "Pay somebody".to_string();
        let description = "Do I pay her?".to_string();
        (msgs, title, description)
    }

    fn pay_somebody_proposal() -> ExecuteMsg {
        let (msgs, title, description) = proposal_info();
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest: None,
        }
    }

    fn text_proposal() -> ExecuteMsg {
        let (_, title, description) = proposal_info();
        ExecuteMsg::Propose {
            title,
            description,
            msgs: vec![],
            latest: None,
        }
    }

    #[test]
    fn test_instantiate_works() {
        let mut app = mock_app(&[]);

        // make a simple group
        let group_addr = instantiate_group(&mut app, vec![member(OWNER, 1)]);
        let flex_id = app.store_code(contract_flex());

        let max_voting_period = Duration::Time(1234567);

        // Zero required weight fails
        let instantiate_msg = InstantiateMsg {
            group_addr: group_addr.to_string(),
            coconut_bandwidth_contract_address: BANDWIDTH_CONTRACT.to_string(),
            coconut_dkg_contract_address: DKG_CONTRACT.to_string(),
            threshold: Threshold::ThresholdQuorum {
                threshold: Decimal::zero(),
                quorum: Decimal::percent(1),
            },
            max_voting_period,
            executor: None,
            proposal_deposit: None,
        };
        let err = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "zero required weight",
                None,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Threshold(cw_utils::ThresholdError::InvalidThreshold {}),
            err.downcast().unwrap()
        );

        // Total weight less than required weight not allowed
        let instantiate_msg = InstantiateMsg {
            group_addr: group_addr.to_string(),
            coconut_bandwidth_contract_address: BANDWIDTH_CONTRACT.to_string(),
            coconut_dkg_contract_address: DKG_CONTRACT.to_string(),
            threshold: Threshold::AbsoluteCount { weight: 100 },
            max_voting_period,
            executor: None,
            proposal_deposit: None,
        };
        let err = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "high required weight",
                None,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Threshold(cw_utils::ThresholdError::UnreachableWeight {}),
            err.downcast().unwrap()
        );

        // All valid
        let instantiate_msg = InstantiateMsg {
            group_addr: group_addr.to_string(),
            coconut_bandwidth_contract_address: BANDWIDTH_CONTRACT.to_string(),
            coconut_dkg_contract_address: DKG_CONTRACT.to_string(),
            threshold: Threshold::AbsoluteCount { weight: 1 },
            max_voting_period,
            executor: None,
            proposal_deposit: None,
        };
        let flex_addr = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "all good",
                None,
            )
            .unwrap();

        // Verify contract version set properly
        let version = query_contract_info(&app.wrap(), flex_addr.clone()).unwrap();
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            version,
        );

        // Get voters query
        let voters: VoterListResponse = app
            .wrap()
            .query_wasm_smart(
                &flex_addr,
                &QueryMsg::ListVoters {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(
            voters.voters,
            vec![VoterDetail {
                addr: OWNER.into(),
                weight: 1
            }]
        );
    }

    #[test]
    fn test_propose_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let required_weight = 4;
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) =
            setup_test_case_fixed(&mut app, required_weight, voting_period, init_funds, false);

        let proposal = pay_somebody_proposal();
        // Only special addresses can propose
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), flex_addr.clone(), &proposal, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // Wrong expiration option fails
        let msgs = match proposal.clone() {
            ExecuteMsg::Propose { msgs, .. } => msgs,
            _ => panic!("Wrong variant"),
        };
        let proposal_wrong_exp = ExecuteMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs,
            latest: Some(Expiration::AtHeight(123456)),
        };
        let err = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal_wrong_exp,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::WrongExpiration {}, err.downcast().unwrap());

        // Proposal from special address works
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "propose"),
                ("sender", BANDWIDTH_CONTRACT),
                ("proposal_id", "1"),
                ("status", "Open"),
            ],
        );
        let res = app
            .execute_contract(Addr::unchecked(DKG_CONTRACT), flex_addr, &proposal, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "propose"),
                ("sender", DKG_CONTRACT),
                ("proposal_id", "2"),
                ("status", "Open"),
            ],
        );
    }

    fn get_tally(app: &App, flex_addr: &str, proposal_id: u64) -> u64 {
        // Get all the voters on the proposal
        let voters = QueryMsg::ListVotes {
            proposal_id,
            start_after: None,
            limit: None,
        };
        let votes: VoteListResponse = app.wrap().query_wasm_smart(flex_addr, &voters).unwrap();
        // Sum the weights of the Yes votes to get the tally
        votes
            .votes
            .iter()
            .filter(|&v| v.vote == Vote::Yes)
            .map(|v| v.weight)
            .sum()
    }

    fn expire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => block.time = block.time.plus_seconds(duration + 1),
                Duration::Height(duration) => block.height += duration + 1,
            };
        }
    }

    fn unexpire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => {
                    block.time =
                        Timestamp::from_nanos(block.time.nanos() - (duration * 1_000_000_000));
                }
                Duration::Height(duration) => block.height -= duration,
            };
        }
    }

    #[test]
    fn test_proposal_queries() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(80),
            quorum: Decimal::percent(20),
        };
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            false,
            None,
            None,
        );

        // create proposal with 1 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        let proposal_id1: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id1,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // another proposal immediately passes
        app.update_block(next_block);
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id2,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // expire them both
        app.update_block(expire(voting_period));

        // add one more open proposal, 2 votes
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        let proposal_id3: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id3,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        let proposed_at = app.block_info();

        // next block, let's query them all... make sure status is properly updated (1 should be rejected in query)
        app.update_block(next_block);
        let list_query = QueryMsg::ListProposals {
            start_after: None,
            limit: None,
        };
        let res: ProposalListResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &list_query)
            .unwrap();
        assert_eq!(3, res.proposals.len());

        // check the id and status are properly set
        let info: Vec<_> = res.proposals.iter().map(|p| (p.id, p.status)).collect();
        let expected_info = vec![
            (proposal_id1, Status::Rejected),
            (proposal_id2, Status::Passed),
            (proposal_id3, Status::Open),
        ];
        assert_eq!(expected_info, info);

        // ensure the common features are set
        let (expected_msgs, expected_title, expected_description) = proposal_info();
        for prop in res.proposals {
            assert_eq!(prop.title, expected_title);
            assert_eq!(prop.description, expected_description);
            assert_eq!(prop.msgs, expected_msgs);
        }

        // reverse query can get just proposal_id3
        let list_query = QueryMsg::ReverseProposals {
            start_before: None,
            limit: Some(1),
        };
        let res: ProposalListResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &list_query)
            .unwrap();
        assert_eq!(1, res.proposals.len());

        let (msgs, title, description) = proposal_info();
        let expected = ProposalResponse {
            id: proposal_id3,
            title,
            description,
            msgs,
            expires: voting_period.after(&proposed_at),
            status: Status::Open,
            threshold: ThresholdResponse::ThresholdQuorum {
                total_weight: 23,
                threshold: Decimal::percent(80),
                quorum: Decimal::percent(20),
            },
            proposer: Addr::unchecked(BANDWIDTH_CONTRACT),
            deposit: None,
        };
        assert_eq!(&expected, &res.proposals[0]);
    }

    #[test]
    fn test_vote_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            false,
            None,
            None,
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Owner with 0 voting power cannot vote
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), flex_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // Only voters can vote
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), flex_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // But voter1 can
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER1),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Open"),
            ],
        );

        // VOTER1 cannot vote again
        let err = app
            .execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::AlreadyVoted {}, err.downcast().unwrap());

        // No/Veto votes have no effect on the tally
        // Compute the current tally
        let tally = get_tally(&app, flex_addr.as_ref(), proposal_id);
        assert_eq!(tally, 1);

        // Cast a No vote
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &no_vote, &[])
            .unwrap();

        // Cast a Veto vote
        let veto_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Veto,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &veto_vote, &[])
            .unwrap();

        // Tally unchanged
        assert_eq!(tally, get_tally(&app, flex_addr.as_ref(), proposal_id));

        let err = app
            .execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::AlreadyVoted {}, err.downcast().unwrap());

        // Expired proposals cannot be voted
        app.update_block(expire(voting_period));
        let err = app
            .execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::Expired {}, err.downcast().unwrap());
        app.update_block(unexpire(voting_period));

        // Powerful voter supports it, so it passes
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER4),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // Passed proposals can still be voted (while they are not expired or executed)
        let res = app
            .execute_contract(Addr::unchecked(VOTER5), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        // Verify
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER5),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed")
            ]
        );

        // query individual votes
        // initial (with 0 weight)
        let voter = BANDWIDTH_CONTRACT.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert_eq!(
            vote.vote.unwrap(),
            VoteInfo {
                proposal_id,
                voter: BANDWIDTH_CONTRACT.into(),
                vote: Vote::Yes,
                weight: 0
            }
        );

        // nay sayer
        let voter = VOTER2.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert_eq!(
            vote.vote.unwrap(),
            VoteInfo {
                proposal_id,
                voter: VOTER2.into(),
                vote: Vote::No,
                weight: 2
            }
        );

        // non-voter
        let voter = SOMEBODY.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert!(vote.vote.is_none());

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Cast a No vote
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &no_vote, &[])
            .unwrap();

        // Powerful voter opposes it, so it rejects
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &no_vote, &[])
            .unwrap();

        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER4),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Rejected"),
            ],
        );

        // Rejected proposals can still be voted (while they are not expired)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER5), flex_addr, &yes_vote, &[])
            .unwrap();

        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER5),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Rejected"),
            ],
        );
    }

    #[test]
    fn test_execute_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            true,
            None,
            None,
        );

        // ensure we have cash to cover the proposal
        let contract_bal = app.wrap().query_balance(&flex_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(10, "BTC"));

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Only Passed can be executed
        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), flex_addr.clone(), &execution, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::WrongExecuteStatus {},
            err.downcast().unwrap()
        );

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER4),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // In passing: Try to close Passed fails
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), flex_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());

        // Execute works. Anybody can execute Passed proposals
        let res = app
            .execute_contract(
                Addr::unchecked(SOMEBODY),
                flex_addr.clone(),
                &execution,
                &[],
            )
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "execute"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // verify money was transfered
        let some_bal = app.wrap().query_balance(SOMEBODY, "BTC").unwrap();
        assert_eq!(some_bal, coin(1, "BTC"));
        let contract_bal = app.wrap().query_balance(&flex_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(9, "BTC"));

        // In passing: Try to close Executed fails
        let err = app
            .execute_contract(Addr::unchecked(OWNER), flex_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());

        // Trying to execute something that was already executed fails
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), flex_addr, &execution, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::WrongExecuteStatus {},
            err.downcast().unwrap()
        );
    }

    #[test]
    fn execute_with_executor_member() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            true,
            Some(nym_multisig_contract_common::state::Executor::Member), // set executor as Member of voting group
            None,
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &vote, &[])
            .unwrap();

        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(
                Addr::unchecked(Addr::unchecked("anyone")), // anyone is not allowed to execute
                flex_addr.clone(),
                &execution,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        app.execute_contract(
            Addr::unchecked(Addr::unchecked(VOTER2)), // member of voting group is allowed to execute
            flex_addr,
            &execution,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn execute_with_executor_only() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            true,
            Some(nym_multisig_contract_common::state::Executor::Only(
                Addr::unchecked(VOTER3),
            )), // only VOTER3 can execute proposal
            None,
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &vote, &[])
            .unwrap();

        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(
                Addr::unchecked(Addr::unchecked("anyone")), // anyone is not allowed to execute
                flex_addr.clone(),
                &execution,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        let err = app
            .execute_contract(
                Addr::unchecked(Addr::unchecked(VOTER1)), // VOTER1 is not allowed to execute
                flex_addr.clone(),
                &execution,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        app.execute_contract(
            Addr::unchecked(Addr::unchecked(VOTER3)), // VOTER3 is allowed to execute
            flex_addr,
            &execution,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn proposal_pass_on_expiration() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = 2000000;
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            Duration::Time(voting_period),
            init_funds,
            true,
            None,
            None,
        );

        // ensure we have cash to cover the proposal
        let contract_bal = app.wrap().query_balance(&flex_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(10, "BTC"));

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Vote it, so it passes after voting period is over
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER3),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Open"),
            ],
        );

        // Wait until the voting period is over.
        app.update_block(|block| {
            block.time = block.time.plus_seconds(voting_period);
            block.height += std::cmp::max(1, voting_period / 5);
        });

        // Proposal should now be passed.
        let prop: ProposalResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &QueryMsg::Proposal { proposal_id })
            .unwrap();
        assert_eq!(prop.status, Status::Passed);

        // Closing should NOT be possible
        let err = app
            .execute_contract(
                Addr::unchecked(SOMEBODY),
                flex_addr.clone(),
                &ExecuteMsg::Close { proposal_id },
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());

        // Execution should now be possible.
        let res = app
            .execute_contract(
                Addr::unchecked(SOMEBODY),
                flex_addr,
                &ExecuteMsg::Execute { proposal_id },
                &[],
            )
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "execute"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );
    }

    #[test]
    fn test_close_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Height(2000000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            true,
            None,
            None,
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Non-expired proposals cannot be closed
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), flex_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::NotExpired {}, err.downcast().unwrap());

        // Expired proposals can be closed
        app.update_block(expire(voting_period));
        let res = app
            .execute_contract(Addr::unchecked(SOMEBODY), flex_addr.clone(), &closing, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "close"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // Trying to close it again fails
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), flex_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn execute_group_changes_from_external() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(20000);
        let (flex_addr, group_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            false,
            None,
            None,
        );

        // VOTER1 starts a proposal to send some tokens (1/4 votes)
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        let prop_status = |app: &App, proposal_id: u64| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };

        // 1/4 votes
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // check current threshold (global)
        let threshold: ThresholdResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &QueryMsg::Threshold {})
            .unwrap();
        let expected_thresh = ThresholdResponse::ThresholdQuorum {
            total_weight: 23,
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        assert_eq!(expected_thresh, threshold);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group
        // updates VOTER2 power to 21 -> with snapshot, vote doesn't pass proposal
        // adds NEWBIE with 2 power -> with snapshot, invalid vote
        // removes VOTER3 -> with snapshot, can vote on proposal
        let newbie: &str = "newbie";
        let update_msg = nym_group_contract_common::msg::ExecuteMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 21), member(newbie, 2)],
        };
        app.execute_contract(Addr::unchecked(OWNER), group_addr, &update_msg, &[])
            .unwrap();

        // check membership queries properly updated
        let query_voter = QueryMsg::Voter {
            address: VOTER3.into(),
        };
        let power: VoterResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &query_voter)
            .unwrap();
        assert_eq!(power.weight, None);

        // proposal still open
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // make a second proposal
        let proposal2 = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal2,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id2,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // VOTER2 can pass this alone with the updated vote (newer height ignores snapshot)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id2,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id2), Status::Passed);

        // VOTER2 can only vote on first proposal with weight of 2 (not enough to pass)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // newbie cannot vote
        let err = app
            .execute_contract(Addr::unchecked(newbie), flex_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // previously removed VOTER3 can still vote, passing the proposal
        app.execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // check current threshold (global) is updated
        let threshold: ThresholdResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &QueryMsg::Threshold {})
            .unwrap();
        let expected_thresh = ThresholdResponse::ThresholdQuorum {
            total_weight: 41,
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        assert_eq!(expected_thresh, threshold);

        // TODO: check proposal threshold not changed
    }

    // uses the power from the beginning of the voting period
    // similar to above - simpler case, but shows that one proposals can
    // trigger the action
    #[test]
    fn execute_group_changes_from_proposal() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let required_weight = 4;
        let voting_period = Duration::Time(20000);
        let (flex_addr, group_addr) =
            setup_test_case_fixed(&mut app, required_weight, voting_period, init_funds, true);

        // Start a proposal to remove VOTER3 from the set
        let update_msg = Cw4GroupContract::new(group_addr)
            .update_members(vec![VOTER3.into()], vec![])
            .unwrap();
        let update_proposal = ExecuteMsg::Propose {
            title: "Kick out VOTER3".to_string(),
            description: "He's trying to steal our money".to_string(),
            msgs: vec![update_msg],
            latest: None,
        };
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &update_proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let update_proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: update_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // next block...
        app.update_block(|b| b.height += 1);

        // VOTER1 starts a proposal to send some tokens
        let cash_proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &cash_proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let cash_proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: cash_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER1), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_ne!(cash_proposal_id, update_proposal_id);

        // query proposal state
        let prop_status = |app: &App, proposal_id: u64| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Open);
        assert_eq!(prop_status(&app, update_proposal_id), Status::Open);

        // next block...
        app.update_block(|b| b.height += 1);

        // Pass and execute first proposal
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: update_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        let execution = ExecuteMsg::Execute {
            proposal_id: update_proposal_id,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &execution, &[])
            .unwrap();

        // ensure that the update_proposal is executed, but the other unchanged
        assert_eq!(prop_status(&app, update_proposal_id), Status::Executed);
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Open);

        // next block...
        app.update_block(|b| b.height += 1);

        // VOTER3 can still pass the cash proposal
        // voting on it fails
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: cash_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Passed);

        // but cannot open a new one
        let cash_proposal = pay_somebody_proposal();
        let err = app
            .execute_contract(
                Addr::unchecked(VOTER3),
                flex_addr.clone(),
                &cash_proposal,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // extra: ensure no one else can call the hook
        let hook_hack = ExecuteMsg::MemberChangedHook(MemberChangedHookMsg {
            diffs: vec![MemberDiff::new(VOTER1, Some(1), None)],
        });
        let err = app
            .execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &hook_hack, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn percentage_handles_group_changes() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        // 51% required, which is 12 of the initial 24
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(20000);
        let (flex_addr, group_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            false,
            None,
            None,
        );

        // VOTER3 starts a proposal to send some tokens (3/12 votes)
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };

        // 3/12 votes
        assert_eq!(prop_status(&app), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group (3 -> 0, 2 -> 9, 0 -> 29) - total = 56, require 29 to pass
        let newbie: &str = "newbie";
        let update_msg = nym_group_contract_common::msg::ExecuteMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 9), member(newbie, 29)],
        };
        app.execute_contract(Addr::unchecked(OWNER), group_addr, &update_msg, &[])
            .unwrap();

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // VOTER2 votes according to original weights: 3 + 2 = 5 / 12 => Open
        // with updated weights, it would be 3 + 9 = 12 / 12 => Passed
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app), Status::Open);

        // new proposal can be passed single-handedly by newbie
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id2,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(newbie), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // check proposal2 status
        let query_prop = QueryMsg::Proposal {
            proposal_id: proposal_id2,
        };
        let prop: ProposalResponse = app
            .wrap()
            .query_wasm_smart(&flex_addr, &query_prop)
            .unwrap();
        assert_eq!(Status::Passed, prop.status);
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn quorum_handles_group_changes() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        // 33% required for quora, which is 8 of the initial 24
        // 50% yes required to pass early (12 of the initial 24)
        let voting_period = Duration::Time(20000);
        let (flex_addr, group_addr) = setup_test_case(
            &mut app,
            Threshold::ThresholdQuorum {
                threshold: Decimal::percent(51),
                quorum: Decimal::percent(33),
            },
            voting_period,
            init_funds,
            false,
            None,
            None,
        );

        // VOTER3 starts a proposal to send some tokens (3 votes)
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };

        // 3/12 votes - not expired
        assert_eq!(prop_status(&app), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group (3 -> 0, 2 -> 9, 0 -> 28) - total = 55, require 28 to pass
        let newbie: &str = "newbie";
        let update_msg = nym_group_contract_common::msg::ExecuteMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 9), member(newbie, 29)],
        };
        app.execute_contract(Addr::unchecked(OWNER), group_addr, &update_msg, &[])
            .unwrap();

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // VOTER2 votes yes, according to original weights: 3 yes, 2 no, 5 total (will fail when expired)
        // with updated weights, it would be 3 yes, 9 yes, 11 total (will pass when expired)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        // not expired yet
        assert_eq!(prop_status(&app), Status::Open);

        // wait until the vote is over, and see it was rejected
        app.update_block(expire(voting_period));
        assert_eq!(prop_status(&app), Status::Rejected);
    }

    #[test]
    fn quorum_enforced_even_if_absolute_threshold_met() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        // 33% required for quora, which is 5 of the initial 15
        // 50% yes required to pass early (8 of the initial 15)
        let voting_period = Duration::Time(20000);
        let (flex_addr, _) = setup_test_case(
            &mut app,
            // note that 60% yes is not enough to pass without 20% no as well
            Threshold::ThresholdQuorum {
                threshold: Decimal::percent(60),
                quorum: Decimal::percent(80),
            },
            voting_period,
            init_funds,
            false,
            None,
            None,
        );

        // create proposal
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER5), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse = app
                .wrap()
                .query_wasm_smart(&flex_addr, &query_prop)
                .unwrap();
            prop.status
        };
        assert_eq!(prop_status(&app), Status::Open);
        app.update_block(|block| block.height += 3);

        // reach 60% of yes votes, not enough to pass early (or late)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap();
        // 9 of 15 is 60% absolute threshold, but less than 12 (80% quorum needed)
        assert_eq!(prop_status(&app), Status::Open);

        // add 3 weight no vote and we hit quorum and this passes
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        app.execute_contract(Addr::unchecked(VOTER3), flex_addr.clone(), &no_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app), Status::Passed);
    }

    #[test]
    fn test_instantiate_with_invalid_deposit() {
        let mut app = App::default();

        let flex_id = app.store_code(contract_flex());

        let group_addr = instantiate_group(
            &mut app,
            vec![Member {
                addr: OWNER.to_string(),
                weight: 10,
            }],
        );

        // Instantiate with an invalid cw20 token.
        let instantiate = InstantiateMsg {
            group_addr: group_addr.to_string(),
            coconut_bandwidth_contract_address: BANDWIDTH_CONTRACT.to_string(),
            coconut_dkg_contract_address: DKG_CONTRACT.to_string(),
            threshold: Threshold::AbsoluteCount { weight: 10 },
            max_voting_period: Duration::Time(10),
            executor: None,
            proposal_deposit: Some(UncheckedDepositInfo {
                amount: Uint128::new(1),
                refund_failed_proposals: true,
                denom: UncheckedDenom::Cw20(group_addr.to_string()),
            }),
        };

        let err: ContractError = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate,
                &[],
                "Bad cw20",
                None,
            )
            .unwrap_err()
            .downcast()
            .unwrap();

        assert_eq!(err, ContractError::Deposit(DepositError::InvalidCw20 {}));

        // Instantiate with a zero amount.
        let instantiate = InstantiateMsg {
            group_addr: group_addr.to_string(),
            coconut_bandwidth_contract_address: BANDWIDTH_CONTRACT.to_string(),
            coconut_dkg_contract_address: DKG_CONTRACT.to_string(),
            threshold: Threshold::AbsoluteCount { weight: 10 },
            max_voting_period: Duration::Time(10),
            executor: None,
            proposal_deposit: Some(UncheckedDepositInfo {
                amount: Uint128::zero(),
                refund_failed_proposals: true,
                denom: UncheckedDenom::Native("native".to_string()),
            }),
        };

        let err: ContractError = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate,
                &[],
                "Bad cw20",
                None,
            )
            .unwrap_err()
            .downcast()
            .unwrap();

        assert_eq!(err, ContractError::Deposit(DepositError::ZeroDeposit {}))
    }

    #[test]
    fn test_cw20_proposal_deposit() {
        let mut app = App::default();

        let cw20_id = app.store_code(contract_cw20());

        let cw20_addr = app
            .instantiate_contract(
                cw20_id,
                Addr::unchecked(OWNER),
                &cw20_base::msg::InstantiateMsg {
                    name: "Token".to_string(),
                    symbol: "TOKEN".to_string(),
                    decimals: 6,
                    initial_balances: vec![
                        Cw20Coin {
                            address: VOTER4.to_string(),
                            amount: Uint128::new(10),
                        },
                        Cw20Coin {
                            address: BANDWIDTH_CONTRACT.to_string(),
                            amount: Uint128::new(10),
                        },
                        Cw20Coin {
                            address: OWNER.to_string(),
                            amount: Uint128::new(10),
                        },
                    ],
                    mint: None,
                    marketing: None,
                },
                &[],
                "Token",
                None,
            )
            .unwrap();

        let (flex_addr, _) = setup_test_case(
            &mut app,
            Threshold::AbsoluteCount { weight: 10 },
            Duration::Height(10),
            vec![],
            true,
            None,
            Some(UncheckedDepositInfo {
                amount: Uint128::new(10),
                denom: UncheckedDenom::Cw20(cw20_addr.to_string()),
                refund_failed_proposals: true,
            }),
        );

        app.execute_contract(
            Addr::unchecked(BANDWIDTH_CONTRACT),
            cw20_addr.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: flex_addr.to_string(),
                amount: Uint128::new(10),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Make a proposal that will pass.
        let proposal = text_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // Make sure the deposit was transfered.
        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: BANDWIDTH_CONTRACT.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::zero());

        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: flex_addr.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::new(10));

        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .unwrap();

        // Make sure the deposit was returned.
        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: VOTER4.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::new(10));

        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: flex_addr.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::zero());

        app.execute_contract(
            Addr::unchecked(BANDWIDTH_CONTRACT),
            cw20_addr.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: flex_addr.to_string(),
                amount: Uint128::new(10),
                expires: None,
            },
            &[],
        )
        .unwrap();

        // Make a proposal that fails.
        let proposal = text_proposal();
        app.execute_contract(
            Addr::unchecked(BANDWIDTH_CONTRACT),
            flex_addr.clone(),
            &proposal,
            &[],
        )
        .unwrap();

        // Check that the deposit was transfered.
        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: flex_addr.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::new(10));

        // Fail the proposal.
        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr.clone(),
            &ExecuteMsg::Vote {
                proposal_id: 2,
                vote: Vote::No,
            },
            &[],
        )
        .unwrap();

        // Expire the proposal.
        app.update_block(|b| b.height += 10);

        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr,
            &ExecuteMsg::Close { proposal_id: 2 },
            &[],
        )
        .unwrap();

        // Make sure the deposit was returned despite the proposal failing.
        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                cw20_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: VOTER4.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::new(10));
    }

    #[test]
    fn proposal_deposit_no_failed_refunds() {
        let mut app = App::default();

        let (flex_addr, _) = setup_test_case(
            &mut app,
            Threshold::AbsoluteCount { weight: 10 },
            Duration::Height(10),
            vec![],
            true,
            None,
            Some(UncheckedDepositInfo {
                amount: Uint128::new(10),
                denom: UncheckedDenom::Native("TOKEN".to_string()),
                refund_failed_proposals: false,
            }),
        );

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: BANDWIDTH_CONTRACT.to_string(),
            amount: vec![Coin {
                amount: Uint128::new(10),
                denom: "TOKEN".to_string(),
            }],
        }))
        .unwrap();

        // Make a proposal that fails.
        let proposal = text_proposal();
        app.execute_contract(
            Addr::unchecked(BANDWIDTH_CONTRACT),
            flex_addr.clone(),
            &proposal,
            &[Coin {
                amount: Uint128::new(10),
                denom: "TOKEN".to_string(),
            }],
        )
        .unwrap();

        // Check that the deposit was transfered.
        let balance = app
            .wrap()
            .query_balance(OWNER, "TOKEN".to_string())
            .unwrap();
        assert_eq!(balance.amount, Uint128::zero());

        // Fail the proposal.
        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr.clone(),
            &ExecuteMsg::Vote {
                proposal_id: 1,
                vote: Vote::No,
            },
            &[],
        )
        .unwrap();

        // Expire the proposal.
        app.update_block(|b| b.height += 10);

        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr,
            &ExecuteMsg::Close { proposal_id: 1 },
            &[],
        )
        .unwrap();

        // Check that the deposit wasn't returned.
        let balance = app
            .wrap()
            .query_balance(OWNER, "TOKEN".to_string())
            .unwrap();
        assert_eq!(balance.amount, Uint128::zero());
    }

    #[test]
    fn test_native_proposal_deposit() {
        let mut app = App::default();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: VOTER4.to_string(),
            amount: vec![Coin {
                amount: Uint128::new(10),
                denom: "TOKEN".to_string(),
            }],
        }))
        .unwrap();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: BANDWIDTH_CONTRACT.to_string(),
            amount: vec![Coin {
                amount: Uint128::new(10),
                denom: "TOKEN".to_string(),
            }],
        }))
        .unwrap();

        let (flex_addr, _) = setup_test_case(
            &mut app,
            Threshold::AbsoluteCount { weight: 10 },
            Duration::Height(10),
            vec![],
            true,
            None,
            Some(UncheckedDepositInfo {
                amount: Uint128::new(10),
                denom: UncheckedDenom::Native("TOKEN".to_string()),
                refund_failed_proposals: true,
            }),
        );

        // Make a proposal that will pass.
        let proposal = text_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(BANDWIDTH_CONTRACT),
                flex_addr.clone(),
                &proposal,
                &[Coin {
                    amount: Uint128::new(10),
                    denom: "TOKEN".to_string(),
                }],
            )
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), flex_addr.clone(), &yes_vote, &[])
            .unwrap();

        // Make sure the deposit was transfered.
        let balance = app
            .wrap()
            .query_balance(flex_addr.clone(), "TOKEN")
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(10));

        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .unwrap();

        // Make sure the deposit was returned.
        let balance = app.wrap().query_balance(VOTER4, "TOKEN").unwrap();
        assert_eq!(balance.amount, Uint128::new(10));

        // Make a proposal that fails.
        let proposal = text_proposal();
        app.execute_contract(
            Addr::unchecked(BANDWIDTH_CONTRACT),
            flex_addr.clone(),
            &proposal,
            &[Coin {
                amount: Uint128::new(10),
                denom: "TOKEN".to_string(),
            }],
        )
        .unwrap();

        let balance = app
            .wrap()
            .query_balance(flex_addr.clone(), "TOKEN")
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(10));

        // Fail the proposal.
        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr.clone(),
            &ExecuteMsg::Vote {
                proposal_id: 2,
                vote: Vote::No,
            },
            &[],
        )
        .unwrap();

        // Expire the proposal.
        app.update_block(|b| b.height += 10);

        app.execute_contract(
            Addr::unchecked(VOTER4),
            flex_addr,
            &ExecuteMsg::Close { proposal_id: 2 },
            &[],
        )
        .unwrap();

        // Make sure the deposit was returned despite the proposal failing.
        let balance = app
            .wrap()
            .query_balance(BANDWIDTH_CONTRACT, "TOKEN")
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(10));
    }
}
