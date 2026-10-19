## Why

`nym-metrics` has no label support: `MetricsController` is a flat `DashMap<String, Metric>` over the unlabelled prometheus types, so every dimension of a metric has to become its own metric name. In the network monitor orchestrator this already costs 18 of its 45 series (three `*_assignments`, three per-kind in-progress gauges, three `*_leases_expired`, two `*_wave_size`, three `performance_submission_*_omitted`, four `bonded_*_nym_nodes`), all of which are one concept repeated per variant. The orchestrator is about to add per-kind capacity metrics, which under the current design would add roughly 21 more names for 7 concepts. `nym-node-metrics` carries the same pattern, including seven histograms that differ only by which registration latency they measure.

The cost is not only series count. Every Grafana panel enumerates metric names by hand instead of using `sum by (kind)`, adding a fourth test kind would require editing every panel, and each per-kind call site carries a `match` whose only job is to pick a metric name.

## What Changes

Labelled families are added **beside** the existing scalar variants, so `Metric` carries both classes:

```
IntCounter(Box<IntCounter>)      IntCounterVec(Box<IntCounterVec>)
IntGauge(Box<IntGauge>)          IntGaugeVec(Box<IntGaugeVec>)
FloatGauge(Box<Gauge>)           FloatGaugeVec(Box<GaugeVec>)
Histogram(Box<Histogram>)        HistogramVec(Box<HistogramVec>)
```

An unlabelled metric keeps the exact prometheus type it has today, so every existing series is unchanged by construction rather than by a proof, and no existing recording path pays for label resolution. That matters because `nymsphinx/chunking` and `mixnet-client` record per message.

- Every recording operation takes label values and dispatches on the variant: an empty slice addresses a scalar metric, and a non-empty one resolves a child of the family it names. The existing unlabelled methods (`inc`, `inc_by`, `set`, `set_float`, `add_to_histogram`, `start_timer`) become thin wrappers that pass an empty slice, so there is **one** implementation per operation rather than parallel labelled and unlabelled ones.
- Supplying label values for a scalar metric, or none for a family, is misuse: it warns and changes nothing, exactly as a type mismatch does today.
- Children resolve through the non-panicking `get_metric_with_label_values`, logging a warning on an arity mismatch. Nothing in this crate panics today and that stays true.
- New labelled constructors and registration helpers take label names alongside name and help.
- **`remove_label_values`**: drop a single child series. `MetricsController` has no removal concept today, which makes any churning label set accumulate series forever.
- **`init_label_values`**: create a child series so it is exposed zeroed from the first scrape, without recording a value. `initialise` currently pre-registers every series precisely so dashboards do not read a first observation as a counter reset, and a labelled family exposes no children until one is touched. A counter or gauge child could be created by writing zero, but a histogram child cannot, because any observation pollutes the distribution.

### Not breaking

Existing constructor signatures are unchanged and still build the same scalar types. Only two call sites outside this crate construct metrics at all, the orchestrator's `prometheus.rs` and `nym-node-metrics`'s `prometheus_wrapper.rs`, and both pass only name and help. Every other consumer goes through the macros or the `maybe_register_and_*` helpers, which are unchanged. The existing `From<IntCounter> for Metric` conversions keep working; the family types gain their own.

`register_metric` needs no change: a `*Vec` collector exposes a single `Desc` for its family, so `fq_name()` already yields the family name and the existing index keying works as is.

### Non-goals

- **Labelled macros.** `inc!`, `set_metric!` and friends take string literals and prepend the package name. Consumers needing labels use `MetricsController` directly, as both wrapper crates already do.
- **Migrating existing metrics onto labels.** This change makes labels available. Collapsing any specific service's repeated variants into labelled families is that service's own change.
- **Reducing the cost of recording as it stands.** Unlabelled recording is untouched by this change. Labelled recording pays a label-values hash and a read-locked child lookup on top, which is comparable to the string hash and `DashMap` read already performed, and small beside the name allocation every caller makes before reaching the registry. If that path ever needs to get faster, the allocation is the target, not the child lookup, and that is a change to how callers address metrics rather than to this interface.

## Capabilities

### New Capabilities

- `metrics-registry`: the shared process-wide metrics registry in `nym-metrics`, covering how metrics are registered, how values are recorded, how labelled families resolve and expose child series, and how misuse is reported.

### Modified Capabilities

None. No existing capability describes `nym-metrics`, and no consumer's observable behaviour changes.

## Impact

- `common/nym-metrics/src/lib.rs`: four new variants, labelled constructors and registration helpers, every recording operation, plus removal and child initialisation. One file, roughly 600 lines today.
- `prometheus` 0.14, already pinned in the workspace. No new dependencies.
- Eleven crates depend on `nym-metrics` (`nym-node`, `nym-node-metrics`, `gateway`, `nymsphinx`, `nymsphinx/chunking`, `mixnet-client`, `mixnode-common`, `wireguard`, `credential-verification`, `statistics`, and the network monitor orchestrator). None require source changes, and none pay anything on the paths they use today, including the per-message recorders in `nymsphinx/chunking` and `mixnet-client`.
- First consumer: the follow-on change `network-monitor-capacity-metrics`, which depends on this landing first.
