use cosmwasm_std::Uint128;
use mixnet_contract::MixNodeBond;
use pyo3::prelude::*;

const ACTIVE_MIX_SET_SIZE: u32 = 1000;

#[pymodule]
fn tokenomics_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compute_mix_rewards_py, m)?)?;

    Ok(())
}

// assign_rewards_to_nodes, Econ_results:344
#[pyfunction]
fn compute_mix_rewards_py(
    pledged: f64,
    delegated: f64,
    total_stake: f64, // total stake delegated across the network, ideally persisted in the blockchain
    performance: f64, // uptime ?
    income_global_mix: f64, // inflation pool
    omega_k: f64, // work_share * number of noes, where workshare is considered uniform 1/k, so it follows this is k in the uniform case
    alpha: f64,   // sybil resistance param - externally defined constant
    k: f64,       // number of desired mixnodes - externally defined constant
) -> f64 {
    // set_lambda_sigma_mixnet, Network_econ:308
    let lambda = (pledged / total_stake).min(1. / k);
    let sigma = ((pledged + delegated) / total_stake).min(1. / k);

    performance * income_global_mix * (sigma * omega_k + alpha * lambda) / (1. + alpha)
}

fn compute_mix_rewards(mix: &MixNodeBond) {
    let k = ACTIVE_MIX_SET_SIZE as f64;
    let one_over_k = 1. / k;
    let one_over_k_uint = (one_over_k * 1_000_000.) as u128;
    // Assume uniform node work distribution for simplicity
    let work_share = one_over_k;
    let omega_k = work_share * k;
    // TODO: Use Coin struct from the Tauri wallet, this must be in the Minor denom it will be much easier then
    
}

// TODO:
// Compute total stake across the entire network, open question is how to handle orphaned delegations
// Compute performance as uptime, uptime can be obtained from the validator API, might be useful to introduce some sugar functions. Validator API is also not available from the contract most likely
// Compute workshare, as either 1/k or node uptime / total uptime, probably best to assume 1/k for now
// k is the active set size
// Where should the inflation pool size be set
