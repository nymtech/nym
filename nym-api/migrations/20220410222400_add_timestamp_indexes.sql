/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE INDEX mixnode_status_timestamp ON mixnode_status(`timestamp`);
CREATE INDEX mixnode_status_id ON mixnode_status(`mixnode_details_id`);

CREATE INDEX gateway_status_timestamp ON gateway_status(`timestamp`);
CREATE INDEX gateway_status_id ON gateway_status(`gateway_details_id`);

CREATE INDEX gateway_details_id on gateway_details(`id`);
CREATE INDEX mixnode_details_id on mixnode_details(`id`);