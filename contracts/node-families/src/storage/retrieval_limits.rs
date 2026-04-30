// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Default page size for paginated family listings when the caller omits `limit`.
pub const FAMILIES_DEFAULT_LIMIT: u32 = 50;

/// Hard cap on the page size for paginated family listings; larger values are clamped.
pub const FAMILIES_MAX_LIMIT: u32 = 100;

/// Default page size for paginated family-member listings when the caller omits `limit`.
pub const FAMILY_MEMBERS_DEFAULT_LIMIT: u32 = 50;

/// Hard cap on the page size for paginated family-member listings; larger values are clamped.
pub const FAMILY_MEMBERS_MAX_LIMIT: u32 = 100;

/// Default page size for paginated pending-invitation listings (both per-family
/// and global) when the caller omits `limit`.
pub const PENDING_INVITATIONS_DEFAULT_LIMIT: u32 = 50;

/// Hard cap on the page size for paginated pending-invitation listings; larger values are clamped.
pub const PENDING_INVITATIONS_MAX_LIMIT: u32 = 100;
