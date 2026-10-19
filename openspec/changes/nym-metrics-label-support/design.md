## Context

`common/nym-metrics` is a roughly 600-line single-file crate wrapping `prometheus` 0.14. `MetricsController` holds a `Registry` plus a `DashMap<String, Metric>` index, and `Metric` is a four-variant enum boxing `IntCounter`, `IntGauge`, `Gauge` and `Histogram`. Every recording operation looks the metric up by name in the index and dispatches on the variant. Nothing in the crate panics: type misuse (calling `set` on a counter, observing on a gauge) logs a warning and is a no-op.

There is no label support of any kind, so a metric with dimensions must be expressed as one metric name per combination. Eleven crates depend on `nym-metrics`. Only two construct metrics directly, `nym-network-monitor-orchestrator`'s `prometheus.rs` and `nym-node-metrics`'s `prometheus_wrapper.rs`, which are near-twins of each other: a `PrometheusMetric` enum, a `to_registrable_metric` mapping, and an `initialise` that pre-registers every series zeroed so that dashboards do not read a first observation as a counter reset. Every other consumer uses the `inc!` / `set_metric!` macros or the `maybe_register_and_*` helpers.

Before this design was written, a probe against `prometheus` 0.14 confirmed that `IntCounterVec::new(opts, &[])` is accepted and that a zero-label family encodes byte-identically to the scalar equivalent, because the text encoder omits an empty label set rather than emitting `{}`. That result is what makes the chosen approach possible.

## Goals / Non-Goals

**Goals:**

- Let a consumer declare a metric family with label dimensions and record against a child series chosen by label values.
- Keep exactly one implementation path for recording, so unlabelled use is the zero-label case rather than a parallel code path.
- Change no existing series on the wire, and require no source changes in any consumer.
- Give the interface the methods a churning label set needs, namely child removal and explicit child creation, rather than discovering them missing in a later change to a shared crate.

**Non-Goals:**

- Labelled variants of the `inc!` / `set_metric!` / `add_histogram_obs!` macros.
- Migrating any consumer's existing metrics onto labels. This change makes labels available; collapsing the orchestrator's or `nym-node-metrics`'s repeated variants is each service's own change.
- Caching resolved child series, or otherwise reducing per-operation cost. The dominant cost on that path is the name allocation every caller already pays, not the child lookup this adds.
- Any change to how metrics are exposed, gathered or encoded.

## Decisions

### Labels go on the existing variants, not into new ones

Each variant's inner type becomes its `*Vec` counterpart: `IntCounter(Box<IntCounterVec>)`, `IntGauge(Box<IntGaugeVec>)`, `FloatGauge(Box<GaugeVec>)`, `Histogram(Box<HistogramVec>)`. An unlabelled metric is a family declared with zero label names.

*Alternative considered:* add `IntCounterVec`, `IntGaugeVec` and `HistogramVec` as new variants beside the scalar ones. Rejected. It doubles the enum, forces every one of the eight `match self` sites to carry arms for both classes, and creates parallel `inc_by` and `inc_by_with_labels` operation paths that each need a warning arm for the other class. It also introduces a whole new misuse category ("labelled operation on an unlabelled metric") that the unified shape does not have at all.

The unified shape is only safe because the encoded output is provably identical, which is why the golden test below is a requirement of this change rather than a nicety.

### Children resolve through the non-panicking accessor

Child lookup uses `get_metric_with_label_values`, which returns a `Result`, and logs a warning on an arity mismatch. The panicking `with_label_values` is not used anywhere.

*Rationale:* the crate's existing contract is that metric misuse degrades to a warning and a no-op. A metrics call panicking in a hot path would be a far worse failure than a missing series, and label arity is a runtime property here because the index is keyed by string.

### The crate stays stringly typed; consumers wrap it

Label values cross the boundary as `&[&str]`. `nym-metrics` sits below every domain type in the workspace and cannot know about `TestKind` or any other enum.

*Rationale:* typed label APIs belong in the consumer's own wrapper, where the domain enum exists and can be converted once at the boundary. The orchestrator will expose something like `inc_for_kind(metric, kind)` over this interface rather than passing strings around at call sites.

### Child series must be creatable without recording a value

`init_label_values(name, labels)` creates a child so it is exposed zeroed from the first scrape.

*Rationale:* a labelled family exposes no children until one is touched, which silently breaks the zeroed-from-first-scrape property that both wrapper crates' `initialise` exists to provide. A counter or gauge child could be created by writing zero, but a histogram child cannot: any observation, including `0.0`, lands in a bucket and pollutes the distribution. So an explicit creation method is required, not merely convenient.

### Removal is part of this change

`remove_label_values(name, labels)` drops a single child.

*Rationale:* `MetricsController` has no removal concept at all today, which is survivable for scalar metrics (there is nothing to remove) and not survivable for labelled ones, where a label set that churns accumulates dead series indefinitely. Adding it later would be a second review of a crate that `nym-node` and `gateway` depend on, for an interface gap that is already known.

### Unlabelled methods stay, as wrappers

`inc`, `inc_by`, `set`, `set_float`, `add_to_histogram` and `start_timer` keep their signatures and delegate to the labelled form with an empty slice.

*Alternative considered:* require every caller to pass label values explicitly. Rejected: it would touch every call site in eleven crates to express "no labels", for no behavioural gain.

## Risks / Trade-offs

**Silent change to exposed metrics.** Every existing series is re-implemented on top of a different prometheus type. A discrepancy would hit all eleven consumers at once and would be invisible until someone noticed a dashboard break. → A golden test encodes one metric of each of the four types both ways, scalar and zero-label family, and asserts the rendered exposition text is byte-identical. The probe already demonstrates this for `IntCounter`; the test covers all four and locks it in.

**Per-operation cost.** Recording gains a label-values hash, a read lock on the children map, and a lookup. That is comparable in size to the string hash and `DashMap` read every operation already performs, and small beside the heap allocation that precedes both: every caller reaches the registry through a freshly allocated name string, either `to_string()` on a wrapper enum or the `format!` inside `prepend_package_name!`. So the increment lands on the cheap part of an already allocation-dominated path. → Accepted, and deliberately not pre-optimised. If per-operation cost on a per-message path such as `nymsphinx/chunking` ever does become a concern, the allocation is the thing to remove, not the label lookup, and that is a separate change to how callers address metrics.

**Label arity errors are runtime warnings, not compile errors.** A caller passing two values to a one-label family gets a log line and a dropped observation. → Inherent to a string-keyed registry, and consistent with how the crate already treats type misuse. Mitigated at the consumer level: each wrapper exposes a typed API whose arity is fixed by its signature, so a mistake has to be made inside the wrapper rather than at every call site.

**Unbounded label sets.** Nothing in the interface prevents a consumer from using a high-cardinality or churning label, which would grow the registry without bound. → `remove_label_values` gives consumers the tool to manage it, but the discipline stays theirs. Worth stating in the capability so the obligation is explicit rather than assumed.

## Migration Plan

Additive in effect: no consumer source changes, no exposition changes, no configuration changes, no data migration. The crate ships in the normal release and consumers pick it up on their next build. Rollback is reverting the crate change, since nothing depends on the new methods until the follow-on orchestrator change lands.

Sequencing matters only in one direction: `network-monitor-capacity-metrics` is the first consumer and must land after this.

## Open Questions

- Whether `nym-node-metrics` should collapse its own repeated variants onto labels. Clear value (seven registration-latency histograms differing only in which operation they time) but out of scope here, and its own change.
