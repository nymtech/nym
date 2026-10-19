## 1. Family types and construction

- [x] 1.1 Add a label-family variant beside each existing `Metric` variant: `IntCounterVec(Box<IntCounterVec>)`, `IntGaugeVec(Box<IntGaugeVec>)`, `FloatGaugeVec(Box<GaugeVec>)`, `HistogramVec(Box<HistogramVec>)`, leaving the four scalar variants exactly as they are
- [x] 1.2 Add `From<..> for Metric` conversions for the four family types, keeping the existing scalar conversions
- [x] 1.3 Keep the existing constructors (`new_int_counter`, `new_int_gauge`, `new_float_gauge`, `new_histogram`) unchanged, still building the scalar types
- [x] 1.4 Add labelled constructors taking an ordered list of label names alongside name and help, building the family types
- [x] 1.5 Add the `as_collector` and `fq_name` arms for the family variants, confirming `fq_name` yields the family name from the family's single descriptor

## 2. Recording

- [x] 2.1 Rewrite each recording operation on `Metric` (`inc`, `inc_by`, `set`, `set_float`, `add_histogram_observation`, `start_timer`) to take label values, keeping ONE implementation per operation that dispatches on the variant: a scalar variant applies the effect directly, a family resolves its child through `get_metric_with_label_values`
- [x] 2.2 Keep the existing type-misuse arms warning and no-op exactly as they do today (set on a counter, observation on a gauge, increment on a histogram), for both classes
- [x] 2.3 Add the class-mismatch arms: label values supplied for a scalar variant, and an arity mismatch against a family, each logging a warning naming the metric and leaving every series unchanged
- [x] 2.4 Add the labelled recording methods on `MetricsController`, each resolving the metric from the name index and delegating to the operation above
- [x] 2.5 Reduce the existing unlabelled `MetricsController` methods to delegations that pass an empty slice, leaving one recording implementation
- [x] 2.6 Add labelled registration helpers on `MetricsController`, mirroring `register_int_gauge` and siblings

## 3. Child lifecycle

- [x] 3.1 Add `init_label_values`, creating a child series so it is exposed from the next scrape without recording an observation or altering a value
- [x] 3.2 Add `remove_label_values`, dropping a single child, warning and leaving all series unchanged when no such child exists
- [x] 3.3 Both warn and change nothing when the name belongs to an unlabelled metric
- [x] 3.4 Document on both methods, in one line each, that the registry bounds nothing and a consumer with churning label values owns removing its dead children

## 4. Tests

- [x] 4.1 An unlabelled metric of each of the four types registers and renders with no label set, through the methods that existed before labels
- [x] 4.2 A family declared with a label name exposes one series per label value, each carrying its own value, under a single `TYPE` line
- [x] 4.3 Children of one family are independent: setting one label value's gauge leaves the other's untouched
- [x] 4.4 An arity mismatch against a family logs and changes nothing, creating no series
- [x] 4.5 Label values supplied for an unlabelled metric log and change nothing
- [x] 4.6 Recording against a name the registry does not hold reports that it did not apply, so the register-and-retry path still works
- [x] 4.7 `init_label_values` on a histogram family exposes the child with a sample count of zero and no bucket populated, and on a counter family exposes it at zero
- [x] 4.8 `remove_label_values` removes only the named child, leaves its siblings exposed, and is a warning-and-no-op for a child that does not exist
- [x] 4.9 A timer started on a family observes against the child its label values address, leaving its siblings at zero

## 5. Verification

- [x] 5.1 `cargo test -p nym-metrics` green
- [x] 5.2 `cargo check` clean for the only crates that can break, namely the two that construct metrics directly (`nym-node-metrics`, `nym-network-monitor-orchestrator`) plus one macro-only consumer to cover the `maybe_register_and_*` path
- [x] 5.3 `cargo fmt` before committing, so the pre-commit hook does not reformat into the commit
