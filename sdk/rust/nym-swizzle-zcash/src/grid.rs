// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Quantized sync ranges: anonymity by collision.
//!
//! A light client that requests exactly the blocks it needs tells the server
//! precisely where its last sync ended. Quantization replaces wallet-specific
//! range edges with network-wide grid boundaries: the start of a queued range
//! is rounded *down* to the grid and the end *up* (capped at the chain tip,
//! which is public), so every wallet resuming anywhere inside the same grid
//! cell emits an identical request. The extra "cover" blocks around the
//! wallet's real range are the price of blending in; [`classify`] tells the
//! wallet what to do with each delivered block so cover is discarded cheaply.
//!
//! The grid spacing is drawn from a ladder that scales with the range length,
//! `S_j = SHARD * 2^j` (nesting with ZIP 318's 144-block anchor grid), with a
//! fixed minimum of [`S_FLOOR`] (about one day of blocks) so frequent syncers
//! don't end up in near-empty cells. **Keep the defaults.** A custom floor or
//! shard size makes your requests recognisably different from every other
//! wallet's — the opposite of the point.
//!
//! Everything in this module is pure arithmetic on block heights: no I/O, no
//! randomness. You can unit-test your integration against it, or use it on
//! its own without the [`sync`](crate::sync) driver.

/// ZIP 318's anchor-grid shard size in blocks; the base rung of the spacing
/// ladder `S_j = SHARD * 2^j`.
pub const SHARD: u64 = 144;

/// The minimum grid spacing in blocks (`SHARD * 8`, roughly one day). Ranges
/// shorter than this still quantize to a full cell of this size. Also the
/// unit the emitted range is split into on the wire (see
/// [`Quantized::requests`]).
pub const S_FLOOR: u64 = 1152;

// The ladder is `SHARD * 2^j`, so the floor must itself be a rung: a floor
// off the doubling family would silently stop nesting with the ZIP 318
// anchor grid. Checked at compile time so it cannot drift.
const _: () = {
    assert!(
        S_FLOOR.is_multiple_of(SHARD),
        "S_FLOOR must be a multiple of SHARD"
    );
    assert!(
        (S_FLOOR / SHARD).is_power_of_two(),
        "S_FLOOR must sit on the SHARD * 2^j ladder"
    );
};

/// How many blocks immediately below a catch-up's resume point are re-fetched
/// so their hashes can be compared against stored state — the reorg check.
/// The emitted range always contains this window; no separate request is made
/// for it (a tiny request just below the resume point would name the resume
/// point exactly).
pub const VERIFY_LOOKAHEAD: u64 = 10;

/// What kind of queued range is being quantized.
///
/// If your wallet SDK queues an explicit verify range (the few blocks below
/// the resume point, re-fetched to detect reorgs), do **not** quantize and
/// fetch it separately — drop it, and tag the adjacent catch-up range as
/// [`Verify`](RangeKind::Verify) instead. The verify window is then absorbed
/// into the catch-up's emitted range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeKind {
    /// A plain scan range. Quantized as-is; no verify window.
    Scan,
    /// A catch-up from a resume point: the [`VERIFY_LOOKAHEAD`] blocks below
    /// the range's start are the reorg-verify window, and the emitted range
    /// is widened (by one whole grid cell if necessary) to contain them.
    Verify,
}

/// A queued half-open scan range `[start, end)`, tagged with its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuedRange {
    /// First height the wallet actually wants.
    pub start: u64,
    /// One past the last height the wallet actually wants.
    pub end: u64,
    /// Whether this range carries the reorg-verify obligation at its start.
    pub kind: RangeKind,
}

impl QueuedRange {
    /// A plain scan range `[start, end)` (e.g. a re-scan positioned by where
    /// the wallet's own notes sit — precisely the kind of range whose *end*
    /// needs the grid as much as its start).
    ///
    /// Panics if `start >= end`.
    pub fn scan(start: u64, end: u64) -> Self {
        assert!(start < end, "empty or inverted range: {start}..{end}");
        Self {
            start,
            end,
            kind: RangeKind::Scan,
        }
    }

    /// The routine catch-up: everything from the wallet's resume point to the
    /// current tip, carrying the reorg-verify window just below the resume
    /// point.
    ///
    /// Panics if `resume_point > tip`.
    pub fn catch_up(resume_point: u64, tip: u64) -> Self {
        assert!(
            resume_point <= tip,
            "resume point {resume_point} is past the tip {tip}"
        );
        Self {
            start: resume_point,
            end: tip + 1,
            kind: RangeKind::Verify,
        }
    }
}

/// The result of quantizing a [`QueuedRange`]: what to put on the wire, what
/// was actually wanted, and where the verify window sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantized {
    requested: (u64, u64),
    emitted: (u64, u64),
    verify_window: Option<(u64, u64)>,
    spacing: u64,
}

impl Quantized {
    /// The half-open range to request on the wire: grid-aligned start,
    /// grid-aligned-or-tip-capped end.
    pub fn emitted(&self) -> (u64, u64) {
        self.emitted
    }

    /// The half-open range the wallet actually queued.
    pub fn requested(&self) -> (u64, u64) {
        self.requested
    }

    /// The half-open reorg-verify window (`None` for plain scan ranges or a
    /// catch-up from height 0). Always contained in the emitted range.
    pub fn verify_window(&self) -> Option<(u64, u64)> {
        self.verify_window
    }

    /// The grid spacing `S` that was selected from the ladder.
    pub fn spacing(&self) -> u64 {
        self.spacing
    }

    /// Number of blocks that will be fetched (emitted range length).
    pub fn emitted_len(&self) -> u64 {
        self.emitted.1 - self.emitted.0
    }

    /// The deterministic sequence of wire requests covering the emitted
    /// range: split at [`S_FLOOR`]-aligned boundaries, ascending, disjoint,
    /// gapless. No randomness anywhere — every wallet resuming in the same
    /// grid cell at the same tip emits **byte-identical requests in
    /// identical order**; that collision is the mechanism, and any
    /// per-wallet variation (sizes, overlap, order) would only add a
    /// distinguishing dimension on top of costing bandwidth.
    ///
    /// The emitted start is a multiple of the spacing (itself a multiple of
    /// [`S_FLOOR`]), so every request is exactly one full [`S_FLOOR`] cell —
    /// except the last, which is shorter only when the emitted end is capped
    /// at the tip. The split exists for retry practicality on long
    /// catch-ups; a failed request can be retried without refetching the
    /// whole range.
    pub fn requests(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        let (start, end) = self.emitted;
        (start..end)
            .step_by(S_FLOOR as usize)
            .map(move |s| (s, (s + S_FLOOR).min(end)))
    }
}

/// Quantize a queued range against the current chain tip.
///
/// 1. `S = max(S_FLOOR, smallest SHARD * 2^j >= range length)`
/// 2. start rounded down to a multiple of `S`
/// 3. end rounded up to a multiple of `S`, capped at `tip + 1` (the tip is
///    public, so a range ending at the tip stays ending at the tip)
/// 4. for [`RangeKind::Verify`], the emitted start is lowered by one whole
///    grid cell if the resume point would otherwise sit within
///    [`VERIFY_LOOKAHEAD`] blocks of it, so the verify window is always
///    inside the emitted range
///
/// Steps 1–3 apply uniformly to every queued range regardless of kind: no
/// classification of where a range's edges came from is needed.
///
/// **Convention: ranges are half-open, and the ladder in step 1 is chosen by
/// half-open length.** This rule is network-wide by construction — two
/// implementations reading it differently would emit different ranges from
/// identical wallet states and partition the collision sets — so the
/// boundary cases are pinned by tests: a range of exactly 1152 blocks takes
/// `S = 1152` and one of 1153 takes `S = 2304`. Note a catch-up scans the
/// tip block too, so a wallet exactly 1152 blocks behind the tip has a
/// 1153-block range and takes `S = 2304`.
///
/// Panics if the range is empty or extends past `tip + 1`.
pub fn quantize(range: QueuedRange, tip: u64) -> Quantized {
    let QueuedRange { start, end, kind } = range;
    assert!(start < end, "empty or inverted range: {start}..{end}");
    assert!(
        end <= tip + 1,
        "queued range {start}..{end} extends past the tip {tip}"
    );

    let spacing = ladder(end - start);
    let mut emitted_start = start - start % spacing;
    // saturating: for realistic heights the product cannot overflow, and if
    // it ever did, the tip cap below restores a sane value
    let emitted_end = end.div_ceil(spacing).saturating_mul(spacing).min(tip + 1);

    let verify_window = match kind {
        RangeKind::Scan => None,
        RangeKind::Verify => {
            let window_start = start.saturating_sub(VERIFY_LOOKAHEAD);
            if window_start < emitted_start {
                // the resume point is too close to the grid boundary: a start
                // this near the wallet's true resume point (or a separate
                // tiny request for the window) would be identifying — take
                // the whole previous cell instead
                emitted_start = emitted_start.saturating_sub(spacing);
            }
            (window_start < start).then_some((window_start, start))
        }
    };

    Quantized {
        requested: (start, end),
        emitted: (emitted_start, emitted_end),
        verify_window,
        spacing,
    }
}

/// The smallest ladder rung `max(S_FLOOR, SHARD * 2^j) >= len`.
fn ladder(len: u64) -> u64 {
    let mut spacing = S_FLOOR;
    while spacing < len {
        spacing = spacing.checked_mul(2).expect("grid spacing overflow");
    }
    spacing
}

/// What the wallet should do with one delivered block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    /// Cover below the requested range: already scanned — discard without
    /// re-scanning or duplicate note insertion.
    CoverBelow,
    /// Inside the reorg-verify window: compare the block hash against stored
    /// state; do not commit the sync until the whole window has passed.
    VerifyWindow,
    /// Inside the requested range: scan normally.
    Requested,
    /// Cover above the requested range: may be new — dedupe against scan
    /// state, then scan what's new.
    CoverAbove,
}

/// Classify a delivered block height against a quantized range.
///
/// Heights outside the emitted range classify by the same rules (below ⇒
/// [`CoverBelow`](Disposition::CoverBelow), above ⇒
/// [`CoverAbove`](Disposition::CoverAbove)); a well-behaved server never
/// sends them.
pub fn classify(height: u64, quantized: &Quantized) -> Disposition {
    if let Some((ws, we)) = quantized.verify_window {
        if (ws..we).contains(&height) {
            return Disposition::VerifyWindow;
        }
    }
    let (start, end) = quantized.requested;
    if height < start {
        Disposition::CoverBelow
    } else if height < end {
        Disposition::Requested
    } else {
        Disposition::CoverAbove
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIP: u64 = 3_000_000;

    #[test]
    fn ladder_scales_with_range_length_and_clamps_to_floor() {
        assert_eq!(ladder(1), S_FLOOR);
        assert_eq!(ladder(S_FLOOR), S_FLOOR);
        assert_eq!(ladder(S_FLOOR + 1), 2 * S_FLOOR);
        assert_eq!(ladder(4 * S_FLOOR), 4 * S_FLOOR);
        assert_eq!(ladder(4 * S_FLOOR + 1), 8 * S_FLOOR);
        // every rung sits on the SHARD * 2^j family — divisibility alone
        // would not catch a floor like 3 * SHARD that never nests
        for len in [1, 100, 5_000, 100_000, 10_000_000] {
            let rung = ladder(len);
            assert_eq!(rung % SHARD, 0);
            assert!(
                (rung / SHARD).is_power_of_two(),
                "rung {rung} off the ladder"
            );
        }
    }

    /// Pins the network-wide range convention: ranges are half-open and the
    /// ladder is chosen by half-open length. An implementation reading the
    /// rule inclusively diverges exactly at rung-multiple gaps and would
    /// partition the collision sets.
    #[test]
    fn half_open_convention_pinned_at_rung_boundaries() {
        // a scan of exactly 1152 blocks takes S = 1152; one more block
        // crosses to the next rung
        assert_eq!(quantize(QueuedRange::scan(0, 1152), TIP).spacing(), 1152);
        assert_eq!(quantize(QueuedRange::scan(0, 1153), TIP).spacing(), 2304);
        assert_eq!(quantize(QueuedRange::scan(0, 2304), TIP).spacing(), 2304);
        assert_eq!(quantize(QueuedRange::scan(0, 2305), TIP).spacing(), 4608);

        // a catch-up scans the tip block too: a wallet exactly 1152 behind
        // has a 1153-block half-open range, so S = 2304 (an inclusive
        // reading of the rule would give 1152 here)
        let q = quantize(QueuedRange::catch_up(TIP - 1152, TIP), TIP);
        assert_eq!(q.spacing(), 2304);
        let q = quantize(QueuedRange::catch_up(TIP - 1151, TIP), TIP);
        assert_eq!(q.spacing(), 1152);

        // on-grid scan ends stay put under the half-open reading
        let q = quantize(QueuedRange::scan(2304, 3456), TIP);
        assert_eq!(q.emitted(), (2304, 3456));
    }

    #[test]
    fn requests_split_at_floor_boundaries_ascending_disjoint_gapless() {
        for &(start, len) in &[
            (500_000u64, 100u64), // one cell
            (500_000, 1153),      // S = 2304: two full cells
            (123_456, 98_765),    // many cells
            (TIP - 500, 501),     // tip-capped: last request short
        ] {
            let q = quantize(QueuedRange::scan(start, start + len), TIP);
            let requests: Vec<_> = q.requests().collect();
            let (es, ee) = q.emitted();

            assert_eq!(requests.first().unwrap().0, es);
            assert_eq!(requests.last().unwrap().1, ee);
            for window in requests.windows(2) {
                // ascending, disjoint, gapless: each request begins exactly
                // where the previous ended
                assert_eq!(window[0].1, window[1].0);
            }
            for &(s, e) in &requests {
                assert_eq!(s % S_FLOOR, 0, "request start off the floor grid");
                assert!(
                    e - s == S_FLOOR || e == ee,
                    "only a tip-capped final request may be short"
                );
            }
        }
    }

    #[test]
    fn start_rounds_down_end_rounds_up() {
        // gap 500 < S_FLOOR, so S = 1152; start 2500 -> 2304, end 3000 -> 3456
        let q = quantize(QueuedRange::scan(2500, 3000), TIP);
        assert_eq!(q.spacing(), 1152);
        assert_eq!(q.emitted(), (2304, 3456));
        assert_eq!(q.requested(), (2500, 3000));
        assert_eq!(q.verify_window(), None);
    }

    #[test]
    fn on_grid_edges_stay_put() {
        let q = quantize(QueuedRange::scan(2304, 3456), TIP);
        assert_eq!(q.emitted(), (2304, 3456));
    }

    #[test]
    fn emitted_end_capped_at_tip() {
        let tip = 10_000;
        // end rounds up to 10368 > tip+1: capped
        let q = quantize(QueuedRange::scan(9_000, 10_000), tip);
        assert_eq!(q.emitted().1, tip + 1);

        // a catch-up ending at the tip emits an end at the tip
        let q = quantize(QueuedRange::catch_up(9_500, tip), tip);
        assert_eq!(q.emitted().1, tip + 1);
    }

    #[test]
    fn same_cell_collision() {
        // two wallets with different resume points in the same cell emit
        // identical ranges at the same tip
        let a = quantize(QueuedRange::catch_up(2_999_000, TIP), TIP);
        let b = quantize(QueuedRange::catch_up(2_999_500, TIP), TIP);
        assert_eq!(a.emitted(), b.emitted());
    }

    #[test]
    fn verify_window_sits_below_resume_point() {
        let resume = 2_999_000;
        let q = quantize(QueuedRange::catch_up(resume, TIP), TIP);
        assert_eq!(q.verify_window(), Some((resume - VERIFY_LOOKAHEAD, resume)));
        // window inside the emitted range
        assert!(q.emitted().0 <= resume - VERIFY_LOOKAHEAD);
    }

    #[test]
    fn resume_point_near_boundary_widens_one_cell() {
        // tip + 1 sits exactly on the grid, and the resume point 5 blocks
        // above the previous boundary: catch-up length < 1152 so S = 1152,
        // and start % S = 5 < VERIFY_LOOKAHEAD forces the widening
        let tip = 1152 * 2604 - 1;
        let boundary = 1152 * 2603;
        let resume = boundary + 5;
        let q = quantize(QueuedRange::catch_up(resume, tip), tip);
        assert_eq!(q.spacing(), 1152);
        assert_eq!(
            q.emitted().0,
            boundary - 1152,
            "emitted start should drop one whole cell"
        );
        assert!(q.emitted().0 <= resume - VERIFY_LOOKAHEAD);
    }

    #[test]
    fn resume_point_exactly_lookahead_above_boundary_does_not_widen() {
        let tip = 1152 * 2604 - 1;
        let boundary = 1152 * 2603;
        let resume = boundary + VERIFY_LOOKAHEAD;
        let q = quantize(QueuedRange::catch_up(resume, tip), tip);
        assert_eq!(q.spacing(), 1152);
        assert_eq!(q.emitted().0, boundary, "window fits exactly; no widening");
    }

    #[test]
    fn catch_up_from_genesis_has_no_window_and_no_widening() {
        let q = quantize(QueuedRange::catch_up(0, TIP), TIP);
        assert_eq!(q.verify_window(), None);
        assert_eq!(q.emitted().0, 0);

        // resume below the lookahead: window truncates at 0; start already 0
        let q = quantize(QueuedRange::catch_up(5, TIP), TIP);
        assert_eq!(q.verify_window(), Some((0, 5)));
        assert_eq!(q.emitted().0, 0);
    }

    #[test]
    fn classification_covers_all_zones() {
        let resume = 2_999_000;
        let q = quantize(
            QueuedRange {
                start: resume,
                end: resume + 400,
                kind: RangeKind::Verify,
            },
            TIP,
        );
        let (es, ee) = q.emitted();
        assert!(es < resume - VERIFY_LOOKAHEAD && ee > resume + 400);

        assert_eq!(classify(es, &q), Disposition::CoverBelow);
        assert_eq!(
            classify(resume - VERIFY_LOOKAHEAD, &q),
            Disposition::VerifyWindow
        );
        assert_eq!(classify(resume - 1, &q), Disposition::VerifyWindow);
        assert_eq!(classify(resume, &q), Disposition::Requested);
        assert_eq!(classify(resume + 399, &q), Disposition::Requested);
        assert_eq!(classify(resume + 400, &q), Disposition::CoverAbove);
        assert_eq!(classify(ee - 1, &q), Disposition::CoverAbove);
    }

    #[test]
    fn scan_range_has_no_verify_zone() {
        let q = quantize(QueuedRange::scan(2500, 3000), TIP);
        assert_eq!(classify(2495, &q), Disposition::CoverBelow);
        assert_eq!(classify(2499, &q), Disposition::CoverBelow);
    }

    /// Structural properties over a sweep of ranges: containment, grid
    /// alignment, bounded extension.
    #[test]
    fn quantization_invariants() {
        let tip = 3_141_592;
        for &(start, len) in &[
            (0u64, 1u64),
            (1, 1),
            (1151, 1),
            (1152, 1),
            (1153, 1),
            (500_000, 100),
            (500_000, 1152),
            (500_000, 1153),
            (123_456, 98_765),
            (3_000_000, 141_592),
            (tip, 1),
            (tip - 1, 2),
        ] {
            for kind in [RangeKind::Scan, RangeKind::Verify] {
                let range = QueuedRange {
                    start,
                    end: start + len,
                    kind,
                };
                let q = quantize(range, tip);
                let (es, ee) = q.emitted();
                let s = q.spacing();

                // emitted contains requested (and the verify window)
                assert!(
                    es <= start && ee >= start + len,
                    "{range:?}: no containment"
                );
                if let Some((ws, we)) = q.verify_window() {
                    assert!(es <= ws && we <= ee, "{range:?}: window escapes");
                    assert!(we - ws <= VERIFY_LOOKAHEAD);
                }

                // grid alignment: start on-grid; end on-grid or tip-capped
                assert_eq!(es % s, 0, "{range:?}: start off-grid");
                assert!(ee % s == 0 || ee == tip + 1, "{range:?}: end off-grid");

                // never past the tip
                assert!(ee <= tip + 1);

                // spacing scales with the gap; extension is bounded
                assert!(s >= S_FLOOR && (s == S_FLOOR || s < 2 * len));
                assert!(
                    q.emitted_len() <= len + 2 * s,
                    "{range:?}: emitted {} blocks for a {len}-block request at spacing {s}",
                    q.emitted_len()
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "past the tip")]
    fn range_past_tip_panics() {
        quantize(QueuedRange::scan(100, 200), 150);
    }

    #[test]
    #[should_panic(expected = "empty or inverted")]
    fn empty_range_panics() {
        QueuedRange::scan(100, 100);
    }
}
