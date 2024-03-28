// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::nyxd::Client;
use crate::support::storage::NymApiStorage;
use anyhow::bail;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub async fn migrate_v3_database(
    storage: &NymApiStorage,
    nyxd_client: &Client,
) -> anyhow::Result<()> {
    if storage.check_v3_migration().await? {
        // we have already run the migration
        return Ok(());
    }

    info!(
        "migrating the database to be compatible with the v3 directory. this might take a while..."
    );

    // get the ids of all the gateways
    let preassigned_ids = nyxd_client
        .get_gateway_ids()
        .await?
        .into_iter()
        .map(|id| (id.identity, id.node_id))
        .collect::<HashMap<_, _>>();
    let contract_gateways = nyxd_client.get_gateways().await?;
    let nym_nodes = nyxd_client.get_nymnodes().await?;

    if preassigned_ids.len() != contract_gateways.len() {
        bail!("CONTRACT DATA CORRUPTION: THE NUMBER OF PREASSIGNED GATEWAY IDS IS DIFFERENT THAN THE NUMBER OF GATEWAYS")
    }

    // assign node_id to every gateway
    let all_known = storage.get_all_known_gateways().await?;
    for gateway in all_known {
        let identity = &gateway.identity;
        debug!("migrating gateway {identity}");
        if let Some(assigned) = preassigned_ids.get(identity) {
            storage
                .set_gateway_node_id(&gateway.identity, *assigned)
                .await?;
            continue;
        };

        // no pre-assigned id, perhaps the operator has already migrated into a nym-node?
        if let Some(nym_node) = nym_nodes
            .iter()
            .find(|n| &n.bond_information.node.identity_key == identity)
        {
            storage
                .set_gateway_node_id(identity, nym_node.node_id())
                .await?;
            continue;
        }

        // check if that gateway is even still bonded
        let bonded = contract_gateways
            .iter()
            .any(|g| &g.gateway.identity_key == identity);

        if !bonded {
            warn!("could not migrate gateway {identity}, as it does not appear to be bonded. all of its data is going to get purged.");
            storage.purge_gateway(gateway.id).await?;
        } else {
            // this is critical issue because it should have never happened
            warn!("could not migrate gateway {identity} even though it's still bonded. something bad has happened!");
            bail!("could not migrate gateway {identity}")
        }
    }

    debug!("making the column not nullable");
    storage.make_node_id_not_null().await?;

    debug!("marking v3 migration as complete");
    storage.set_v3_migration_completion().await?;

    Ok(())
}
