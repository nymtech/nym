// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Headless **sandbox** smoke for the node-families real-IPC layer
//! (node-families-real-ipc tasks 4.1 read smoke / 5.2-5.3 guarded writes).
//!
//! GUI automation of the real wallet can't run on macOS (tauri-driver is
//! Linux/Windows-only; Playwright can't drive the native webview), so this
//! exercises the exact `validator-client` calls the Tauri commands in
//! `operations/families/` wrap, against the contract deployed to sandbox
//! (address bundled in `nym-wallet-types/src/network/sandbox.rs`).
//!
//! The funded sandbox account mnemonic is read from `.env` at runtime
//! (`TAURI-WALLET-MNEMONIC`); it is **never** printed, logged, or written
//! anywhere. Run from the `nym-wallet/` directory so `.env` is found:
//!
//!   # read-only smoke (safe, no state change) — task 4.1
//!   cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke
//!
//!   # + guarded write journey (create → rename → disband, with cleanup) — task 5.2
//!   cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke -- --write
//!
//! The write journey only touches a throwaway family this account creates and
//! disbands within the run; it refuses to start if the account already owns a
//! family (so it never clobbers pre-existing state).

use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

use bip39::Mnemonic;
use nym_config::defaults::NymNetworkDetails;
use nym_mixnet_contract_common::NodeId;
use nym_node_families_contract_common::{Config as FamilyConfig, NodeFamilyId};
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, NodeFamiliesQueryClient, NodeFamiliesSigningClient, NymContractsProvider,
    PagedNodeFamiliesQueryClient,
};
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::{AccountId, Coin, CosmWasmClient};
use nym_wallet_types::network::Network as WalletNetwork;

type Smoke = Result<(), Box<dyn Error>>;
type Client = nym_validator_client::DirectSigningHttpRpcValidatorClient;

fn tx(label: &str, res: ExecuteResult) {
    println!("    {label} → tx {}", res.transaction_hash);
}

/// Read a value for `key` from `.env` (handles hyphenated keys, which not every
/// dotenv loader exports into the process env) or fall back to the process env.
/// Secret values are returned for use but never printed by this harness.
fn read_env(key: &str) -> Option<String> {
    [".env", "nym-wallet/.env", "../.env"]
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let line = line.trim();
                if line.starts_with('#') {
                    return None;
                }
                line.strip_prefix(&format!("{key}="))
                    .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
            })
        })
        .or_else(|| std::env::var(key).ok())
        .filter(|v| !v.is_empty())
}

/// Load a mnemonic from the first of `keys` present in `.env`.
fn mnemonic_from(keys: &[&str]) -> Result<Mnemonic, Box<dyn Error>> {
    let phrase = keys
        .iter()
        .find_map(|k| read_env(k))
        .ok_or_else(|| format!("none of {keys:?} found in .env (run from the nym-wallet/ dir)"))?;
    Ok(Mnemonic::from_str(phrase.trim())?)
}

fn build_client(mnemonic: Mnemonic) -> Result<Client, Box<dyn Error>> {
    let network: NymNetworkDetails = WalletNetwork::SANDBOX.into();
    let config = nym_validator_client::Config::try_from_nym_network_details(&network)?;
    Ok(nym_validator_client::Client::new_signing(config, mnemonic)?)
}

/// The numeric node_id an account controls (nym-node, else legacy mixnode), if any.
async fn controlled_node(client: &Client) -> Result<Option<NodeId>, Box<dyn Error>> {
    let me = client.nyxd.address();
    if let Some(d) = client.nyxd.get_owned_nymnode(&me).await?.details {
        return Ok(Some(d.bond_information.node_id));
    }
    if let Some(m) = client.nyxd.get_owned_mixnode(&me).await?.mixnode_details {
        return Ok(Some(m.bond_information.mix_id));
    }
    Ok(None)
}

/// Print the on-chain family/node state of the owner + operator accounts so we
/// can pick the right write flow before mutating anything.
async fn accounts_state() -> Smoke {
    let owner = build_client(mnemonic_from(&["FAMILY_OWNER_MNEMONIC"])?)?;
    let operator = build_client(mnemonic_from(&["ACCOUNT_WITH_BONDED_NODE_MNEMONIC"])?)?;

    let o_addr = owner.nyxd.address();
    let o_family = owner.nyxd.get_family_by_owner(&o_addr).await?.family;
    println!("\n=== ACCOUNTS STATE ===");
    println!("FAMILY_OWNER            = {o_addr}");
    match &o_family {
        Some(f) => println!(
            "  owns family id={} name={:?} members={}",
            f.id, f.name, f.members
        ),
        None => println!("  owns no family"),
    }

    let p_addr = operator.nyxd.address();
    let node = controlled_node(&operator).await?;
    println!("ACCOUNT_WITH_BONDED_NODE = {p_addr}");
    match node {
        Some(id) => {
            let membership = operator.nyxd.get_family_membership(id).await?.family_id;
            println!("  controls node_id={id}, current family membership = {membership:?}");
        }
        None => println!("  controls no node"),
    }
    Ok(())
}

/// Read the contract `Config` straight from raw state (the same path
/// `get_family_config` uses — there is no `GetConfig` smart query).
async fn read_config(
    client: &nym_validator_client::DirectSigningHttpRpcValidatorClient,
) -> Result<FamilyConfig, Box<dyn Error>> {
    let contract = client
        .nyxd
        .node_families_contract_address()
        .ok_or("node_families_contract_address is not set for SANDBOX")?
        .clone();
    let raw = client
        .nyxd
        .query_contract_raw(&contract, b"config".to_vec())
        .await?;
    Ok(serde_json::from_slice(&raw)?)
}

async fn read_smoke(client: &nym_validator_client::DirectSigningHttpRpcValidatorClient) -> Smoke {
    println!("\n=== READ SMOKE (task 4.1) ===");

    let config = read_config(client).await?;
    println!(
        "config: create_family_fee={} {}, name_limit={}, desc_limit={}, default_invite_validity={}s",
        config.create_family_fee.amount,
        config.create_family_fee.denom,
        config.family_name_length_limit,
        config.family_description_length_limit,
        config.default_invitation_validity_secs,
    );

    let families = client.nyxd.get_all_families().await?;
    println!(
        "get_all_families → {} family/families on sandbox",
        families.len()
    );
    for f in &families {
        println!(
            "  • id={} name={:?} owner={} members={} created_at={}",
            f.id, f.name, f.owner, f.members, f.created_at
        );
        let members = client.nyxd.get_all_family_members_for_family(f.id).await?;
        for m in &members {
            println!(
                "      member node_id={} joined_at={}",
                m.node_id, m.membership.joined_at
            );
        }
        let pending = client
            .nyxd
            .get_all_pending_invitations_for_family(f.id)
            .await?;
        for p in &pending {
            println!(
                "      pending invite node_id={} expires_at={} expired={}",
                p.invitation.node_id, p.invitation.expires_at, p.expired
            );
        }
    }

    let me = client.nyxd.address();
    let owned = client.nyxd.get_family_by_owner(&me).await?;
    match owned.family {
        Some(f) => println!("this account owns family id={} ({:?})", f.id, f.name),
        None => println!("this account does not currently own a family"),
    }

    println!("read smoke OK ✅");
    Ok(())
}

async fn write_journey(client: &Client) -> Smoke {
    println!("\n=== GUARDED WRITE JOURNEY (tasks 5.2 + 5.3: full owner + operator lifecycle) ===");

    let me = client.nyxd.address();

    // Guard: never clobber a pre-existing family owned by this account.
    if let Some(existing) = client.nyxd.get_family_by_owner(&me).await?.family {
        return Err(format!(
            "account already owns family id={} ({:?}); refusing to run the write journey — \
             disband it manually first if this is the throwaway test account",
            existing.id, existing.name
        )
        .into());
    }

    // Resolve the numeric node_id this account controls. The account acts as
    // BOTH family owner (invite/kick/revoke/disband) and node operator
    // (accept/reject/leave), so a single funded account drives the whole
    // lifecycle — covering all 9 execute commands.
    // The member-management commands (invite/accept/reject/kick/revoke/leave)
    // need a numeric node_id this account controls; the account is both family
    // owner and node operator, so one account can drive the whole lifecycle.
    // If no node is bonded we still run the owner-only subset (create/update/
    // disband) and skip the member steps with a clear notice.
    let nym_node = client.nyxd.get_owned_nymnode(&me).await?.details;
    let legacy_mixnode = client.nyxd.get_owned_mixnode(&me).await?.mixnode_details;
    let controlled: Option<NodeId> = match (&nym_node, &legacy_mixnode) {
        (Some(d), _) => Some(d.bond_information.node_id),
        (_, Some(m)) => Some(m.bond_information.mix_id),
        (None, None) => None,
    };
    match controlled {
        Some(id) => println!(
            "controlled node_id = {id} ({})",
            if nym_node.is_some() {
                "nym-node"
            } else {
                "legacy mixnode"
            }
        ),
        None => println!(
            "⚠️  account controls no node on the sandbox mixnet contract — running the owner-only \
             subset (create/update/disband); skipping invite/accept/reject/kick/revoke/leave"
        ),
    }

    let config = read_config(client).await?;
    let creation_fee: Vec<Coin> = vec![config.create_family_fee.into()];

    println!("\n[1] create_family attaching {} ...", creation_fee[0]);
    tx(
        "create_family",
        client
            .nyxd
            .create_family(
                "smoke-test-family".to_string(),
                "throwaway family created by sandbox_families_smoke".to_string(),
                None,
                creation_fee,
            )
            .await?,
    );
    let fid = poll_for_owned_family(client, &me).await?.id;
    println!("    → family id={fid}");

    println!("\n[2] update_family (rename) ...");
    tx(
        "update_family",
        client
            .nyxd
            .update_family(Some("smoke-test-renamed".to_string()), None, None)
            .await?,
    );

    if let Some(node_id) = controlled {
        println!("\n[3] invite_to_family → revoke_family_invitation (owner revokes a pending invite) ...");
        tx(
            "invite_to_family",
            client.nyxd.invite_to_family(node_id, None, None).await?,
        );
        poll_pending(client, fid, node_id, true).await?;
        tx(
            "revoke_family_invitation",
            client.nyxd.revoke_family_invitation(node_id, None).await?,
        );
        poll_pending(client, fid, node_id, false).await?;

        println!("\n[4] invite_to_family → reject_family_invitation (operator rejects) ...");
        tx(
            "invite_to_family",
            client.nyxd.invite_to_family(node_id, None, None).await?,
        );
        poll_pending(client, fid, node_id, true).await?;
        tx(
            "reject_family_invitation",
            client
                .nyxd
                .reject_family_invitation(fid, node_id, None)
                .await?,
        );
        poll_pending(client, fid, node_id, false).await?;

        println!("\n[5] invite → accept_family_invitation → leave_family (operator joins then leaves) ...");
        tx(
            "invite_to_family",
            client.nyxd.invite_to_family(node_id, None, None).await?,
        );
        poll_pending(client, fid, node_id, true).await?;
        tx(
            "accept_family_invitation",
            client
                .nyxd
                .accept_family_invitation(fid, node_id, None)
                .await?,
        );
        poll_membership(client, node_id, Some(fid)).await?;
        tx(
            "leave_family",
            client.nyxd.leave_family(node_id, None).await?,
        );
        poll_membership(client, node_id, None).await?;

        println!("\n[6] invite → accept → kick_from_family (owner kicks the member) ...");
        tx(
            "invite_to_family",
            client.nyxd.invite_to_family(node_id, None, None).await?,
        );
        poll_pending(client, fid, node_id, true).await?;
        tx(
            "accept_family_invitation",
            client
                .nyxd
                .accept_family_invitation(fid, node_id, None)
                .await?,
        );
        poll_membership(client, node_id, Some(fid)).await?;
        tx(
            "kick_from_family",
            client.nyxd.kick_from_family(node_id, None).await?,
        );
        poll_membership(client, node_id, None).await?;
    }

    println!("\n[7] disband_family (cleanup) ...");
    tx("disband_family", client.nyxd.disband_family(None).await?);
    poll_until_no_owned_family(client, &me).await?;

    if controlled.is_some() {
        println!("\nwrite journey OK ✅ — all 9 execute commands exercised; state cleaned up");
    } else {
        println!(
            "\nwrite journey OK ✅ — owner subset (create/update/disband) exercised; \
             state cleaned up. Bond a node to this account to also cover the 6 member commands"
        );
    }
    Ok(())
}

/// On-chain state isn't readable until the block commits; poll briefly.
async fn poll_for_owned_family(
    client: &Client,
    owner: &AccountId,
) -> Result<nym_node_families_contract_common::NodeFamily, Box<dyn Error>> {
    for _ in 0..10 {
        if let Some(f) = client.nyxd.get_family_by_owner(owner).await?.family {
            return Ok(f);
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err("timed out waiting for the created family to appear on chain".into())
}

async fn poll_until_no_owned_family(client: &Client, owner: &AccountId) -> Smoke {
    for _ in 0..10 {
        if client
            .nyxd
            .get_family_by_owner(owner)
            .await?
            .family
            .is_none()
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err("timed out waiting for the family to be disbanded".into())
}

/// Wait until a pending invitation for `(fid, node_id)` is present/absent.
async fn poll_pending(client: &Client, fid: NodeFamilyId, node_id: NodeId, want: bool) -> Smoke {
    for _ in 0..10 {
        let present = client
            .nyxd
            .get_pending_invitation(fid, node_id)
            .await?
            .invitation
            .is_some();
        if present == want {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(format!("timed out waiting for pending invitation present={want}").into())
}

/// Wait until `node_id`'s membership equals `want` (`Some(fid)` joined / `None` not a member).
async fn poll_membership(client: &Client, node_id: NodeId, want: Option<NodeFamilyId>) -> Smoke {
    for _ in 0..10 {
        if client.nyxd.get_family_membership(node_id).await?.family_id == want {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(format!("timed out waiting for membership = {want:?}").into())
}

/// Two-account member/operator journey (task 5.3) against the owner's EXISTING
/// family — owner = `FAMILY_OWNER` (invite/revoke/kick), operator =
/// `ACCOUNT_WITH_BONDED_NODE` (accept/reject/leave). Exercises all 6 member
/// commands and restores the node's original membership at the end, so a
/// real pre-existing family is left exactly as it was found.
async fn member_journey(owner: &Client, operator: &Client) -> Smoke {
    println!("\n=== MEMBER / OPERATOR JOURNEY (task 5.3) ===");

    let owner_addr = owner.nyxd.address();
    let fid = owner
        .nyxd
        .get_family_by_owner(&owner_addr)
        .await?
        .family
        .ok_or("FAMILY_OWNER owns no family to run the member journey against")?
        .id;
    let node_id = controlled_node(operator)
        .await?
        .ok_or("ACCOUNT_WITH_BONDED_NODE controls no node")?;
    let initial = operator
        .nyxd
        .get_family_membership(node_id)
        .await?
        .family_id;
    println!("owner={owner_addr}\noperator node_id={node_id}, family={fid}, initial membership={initial:?} (restored at end)");

    // Baseline: clear any stray pending invite, then make node a clean member of `fid`.
    if owner
        .nyxd
        .get_pending_invitation(fid, node_id)
        .await?
        .invitation
        .is_some()
    {
        tx(
            "revoke (baseline)",
            owner.nyxd.revoke_family_invitation(node_id, None).await?,
        );
        poll_pending(owner, fid, node_id, false).await?;
    }
    if operator
        .nyxd
        .get_family_membership(node_id)
        .await?
        .family_id
        != Some(fid)
    {
        tx(
            "invite (baseline)",
            owner.nyxd.invite_to_family(node_id, None, None).await?,
        );
        poll_pending(owner, fid, node_id, true).await?;
        tx(
            "accept (baseline)",
            operator
                .nyxd
                .accept_family_invitation(fid, node_id, None)
                .await?,
        );
        poll_membership(operator, node_id, Some(fid)).await?;
    }

    println!("\n[a] kick_from_family (owner removes the member) ...");
    tx(
        "kick_from_family",
        owner.nyxd.kick_from_family(node_id, None).await?,
    );
    poll_membership(operator, node_id, None).await?;

    println!("\n[b] invite_to_family → reject_family_invitation (operator rejects) ...");
    tx(
        "invite_to_family",
        owner.nyxd.invite_to_family(node_id, None, None).await?,
    );
    poll_pending(owner, fid, node_id, true).await?;
    tx(
        "reject_family_invitation",
        operator
            .nyxd
            .reject_family_invitation(fid, node_id, None)
            .await?,
    );
    poll_pending(owner, fid, node_id, false).await?;

    println!("\n[c] invite_to_family → revoke_family_invitation (owner revokes) ...");
    tx(
        "invite_to_family",
        owner.nyxd.invite_to_family(node_id, None, None).await?,
    );
    poll_pending(owner, fid, node_id, true).await?;
    tx(
        "revoke_family_invitation",
        owner.nyxd.revoke_family_invitation(node_id, None).await?,
    );
    poll_pending(owner, fid, node_id, false).await?;

    println!(
        "\n[d] invite → accept_family_invitation → leave_family (operator joins then leaves) ..."
    );
    tx(
        "invite_to_family",
        owner.nyxd.invite_to_family(node_id, None, None).await?,
    );
    poll_pending(owner, fid, node_id, true).await?;
    tx(
        "accept_family_invitation",
        operator
            .nyxd
            .accept_family_invitation(fid, node_id, None)
            .await?,
    );
    poll_membership(operator, node_id, Some(fid)).await?;
    tx(
        "leave_family",
        operator.nyxd.leave_family(node_id, None).await?,
    );
    poll_membership(operator, node_id, None).await?;

    // Restore the node's original membership so a real family is left unchanged.
    println!("\n[restore] returning node {node_id} to its initial membership {initial:?} ...");
    match initial {
        Some(f) if f == fid => {
            tx("invite_to_family", owner.nyxd.invite_to_family(node_id, None, None).await?);
            poll_pending(owner, fid, node_id, true).await?;
            tx("accept_family_invitation", operator.nyxd.accept_family_invitation(fid, node_id, None).await?);
            poll_membership(operator, node_id, Some(fid)).await?;
        }
        None => poll_membership(operator, node_id, None).await?,
        Some(other) => println!(
            "  ⚠️ node was originally in family {other} (not {fid}); leaving it free — re-add manually if needed"
        ),
    }

    println!("\nmember journey OK ✅ — all 6 member commands exercised; node membership restored");
    Ok(())
}

/// Read-only: report which (if any) node an arbitrary account controls on the
/// sandbox mixnet contract. Signing identity is irrelevant — these are queries.
async fn bond_check(client: &Client, addr: &str) -> Smoke {
    let account = AccountId::from_str(addr)?;
    let nym_node = client.nyxd.get_owned_nymnode(&account).await?.details;
    let legacy = client
        .nyxd
        .get_owned_mixnode(&account)
        .await?
        .mixnode_details;
    let gateway = client.nyxd.get_owned_gateway(&account).await?.gateway;

    println!("\n=== BOND CHECK: {addr} ===");
    println!(
        "mixnet contract: {}",
        client
            .nyxd
            .mixnet_contract_address()
            .map(|a| a.to_string())
            .unwrap_or_default()
    );
    match (&nym_node, &legacy) {
        (Some(d), _) => println!(
            "✅ controls nym-node → node_id = {}",
            d.bond_information.node_id
        ),
        (_, Some(m)) => println!(
            "✅ controls legacy mixnode → node_id (mix_id) = {}",
            m.bond_information.mix_id
        ),
        (None, None) => println!("❌ controls no nym-node and no mixnode"),
    }
    println!(
        "   gateway: {}",
        if gateway.is_some() {
            "yes (gateways can't be family members)"
        } else {
            "none"
        }
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Smoke {
    let args: Vec<String> = std::env::args().collect();
    let do_write = args.iter().any(|a| a == "--write");
    let do_accounts = args.iter().any(|a| a == "--accounts");
    let do_member = args.iter().any(|a| a == "--member");
    let bond_check_addr = args
        .iter()
        .position(|a| a == "--bond-check")
        .and_then(|i| args.get(i + 1))
        .cloned();

    // These only need the owner/operator keys, not the primary account.
    if do_accounts {
        return accounts_state().await;
    }
    if do_member {
        let owner = build_client(mnemonic_from(&["FAMILY_OWNER_MNEMONIC"])?)?;
        let operator = build_client(mnemonic_from(&["ACCOUNT_WITH_BONDED_NODE_MNEMONIC"])?)?;
        return member_journey(&owner, &operator).await;
    }

    let client = build_client(mnemonic_from(&[
        "TAURI-WALLET-MNEMONIC",
        "TAURI_WALLET_MNEMONIC",
    ])?)?;

    println!("connected to SANDBOX as {}", client.nyxd.address());
    println!(
        "node_families_contract_address = {:?}",
        client.nyxd.node_families_contract_address()
    );

    if let Some(addr) = bond_check_addr {
        bond_check(&client, &addr).await?;
        return Ok(());
    }

    read_smoke(&client).await?;

    if do_write {
        write_journey(&client).await?;
    } else {
        println!("\n(skipping write journey — pass `-- --write` to run create → rename → disband)");
    }

    println!("\nDone.");
    Ok(())
}
