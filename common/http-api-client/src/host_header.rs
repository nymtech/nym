//! Lets [`crate::Client::apply_hosts_to_req`] tell a `Host` header it is still managing itself
//! (e.g. across a retry, after a host rotation) apart from one a caller has set or changed, so
//! the caller's value is never silently clobbered or stripped.
//!
//! There's no `reqwest::Request` extension point to stash that fact on, so it's recorded in a
//! marker header instead: [`MANAGED_MARKER_HEADER`], stripped before the request is actually
//! sent (see `Client::send`) and never visible on the wire.

use reqwest::header::{HOST, HeaderValue};

pub(crate) const MANAGED_MARKER_HEADER: &str = "NYM-INTERNAL-MANAGED-HOST";
const UNSET: &str = "unset";

/// The request's effective `Host` value. `reqwest::RequestBuilder::header` appends rather than
/// replaces, so if a caller adds a `Host` header on top of one we already set, the last one --
/// the caller's -- is the one that counts.
fn effective(r: &reqwest::Request) -> Option<&HeaderValue> {
    r.headers().get_all(HOST).iter().next_back()
}

/// Has the caller set (or changed) the `Host` header since we last managed it ourselves?
pub(crate) fn overridden_by_caller(r: &reqwest::Request) -> bool {
    let current = effective(r).and_then(|v| v.to_str().ok());
    let last_managed = r
        .headers()
        .get(MANAGED_MARKER_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|marker| *marker != UNSET);

    current != last_managed
}

/// Sets the request's `Host` header to `host` (or removes it if `None`), and records that we
/// did so, so a later call recognizes its own work.
pub(crate) fn set(r: &mut reqwest::Request, host: Option<&str>) {
    r.headers_mut().remove(HOST);
    if let Some(value) = host.and_then(|h| HeaderValue::from_str(h).ok()) {
        r.headers_mut().insert(HOST, value);
    }

    if let Ok(marker) = HeaderValue::from_str(host.unwrap_or(UNSET)) {
        r.headers_mut().insert(MANAGED_MARKER_HEADER, marker);
    }
}

/// Collapses a caller-overridden `Host` header down to its single effective value, so the
/// outgoing request never carries two `Host` headers -- not just ambiguous, but a
/// request-smuggling hazard.
pub(crate) fn collapse_override(r: &mut reqwest::Request) {
    if let Some(value) = effective(r).cloned() {
        r.headers_mut().remove(HOST);
        r.headers_mut().insert(HOST, value);
    }
}
