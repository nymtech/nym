// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) mod handlers;

// pub(crate) mod routes;
// mod unstable_routes;

// /// Merges the routes with http information and returns it to Rocket for serving
// pub(crate) fn nym_node_routes_deprecated(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
//     openapi_get_routes_spec![
//         settings:
//         routes::get_gateways_described,
//         routes::get_mixnodes_described,
//     ]
// }
//
// pub(crate) fn nym_node_routes_next(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
//     openapi_get_routes_spec![
//         settings:
//         unstable_routes::nodes_basic,
//         unstable_routes::nodes_expanded,
//         unstable_routes::nodes_detailed,
//         unstable_routes::gateways_basic,
//         unstable_routes::gateways_expanded,
//         unstable_routes::gateways_detailed,
//         unstable_routes::mixnodes_basic,
//         unstable_routes::mixnodes_expanded,
//         unstable_routes::mixnodes_detailed,
//         routes::all_described_nodes,
//         routes::node_description,
//         routes::node_annotation_by_identity,
//         routes::node_annotation
//     ]
// }
