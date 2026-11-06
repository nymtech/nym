/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Results no longer go to nym-api, so nothing tracks how far each kind's stream was submitted.
-- They reach the performance contract through the materialiser, which reads the stored runs by
-- window instead.
DROP TABLE submission_watermark;
