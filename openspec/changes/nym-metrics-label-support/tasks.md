## 1. Family types and construction

- [ ] 1.1 Change each `Metric` variant's inner type to its label-family counterpart: `IntCounter(Box<IntCounterVec>)`, `IntGauge(Box<IntGaugeVec>)`, `FloatGauge(Box<GaugeVec>)`, `Histogram(Box<HistogramVec>)`
- [ ] 1.2 Retarget the `From<..> for Metric` conversions onto the family types, or drop any that no longer has a caller inside the crate
- [ ] 1.3 Keep the existing constructors (`new_int_counter`, `new_int_gauge`, `new_float_gauge`, `new_histogram`) at their current signatures, now building a family with no label names
- [ ] 1.4 Add labelled constructors taking an ordered list of label names alongside name and help
- [ ] 1.5 Update the `as_collector` and `fq_name` arms for the new inner types, confirming `fq_name` still yields the family name from the family's single descriptor

## 2. Recording

- [ ] 2.1 Rewrite each recording operation on `Metric` (`inc`, `inc_by`, `set`, `set_float`, `add_histogram_observation`, `start_timer`) to take label values and resolve the child through `get_metric_with_label_values`, logging a warning naming the family on an arity mismatch and leaving every series unchanged
- [ ] 2.2 Keep the existing type-misuse arms warning and no-op exactly as they do today (set on a counter, observation on a gauge, increment on a histogram)
- [ ] 2.3 Add the labelled recording methods on `MetricsController`, each resolving the family from the name index and delegating to the operation above
- [ ] 2.4 Reduce the existing unlabelled `MetricsController` methods to delegations that pass an empty slice, leaving one recording implementation
- [ ] 2.5 Add labelled registration helpers on `MetricsController`, mirroring `register_int_gauge` and siblings

## 3. Child lifecycle

- [ ] 3.1 Add `init_label_values`, creating a child series so it is exposed from the next scrape without recording an observation or altering a value
- [ ] 3.2 Add `remove_label_values`, dropping a single child, warning and leaving all series unchanged when no such child exists
- [ ] 3.3 Document on both methods, in one line each, that the registry bounds nothing and a consumer with churning label values owns removing its dead children

## 4. Tests

- [ ] 4.1 Golden test: for each of the four metric types, register and record through `MetricsController` both as a no-label family and as the pre-existing scalar form, and assert the rendered exposition text is byte-identical
- [ ] 4.2 A family declared with a label name exposes one series per label value, each carrying its own value, under a single `TYPE` line
- [ ] 4.3 Children of one family are independent: setting one label value's gauge leaves the other's untouched
- [ ] 4.4 An arity mismatch logs and changes nothing, creating no series
- [ ] 4.5 Recording against a name the registry does not hold reports that it did not apply, so the register-and-retry path still works
- [ ] 4.6 `init_label_values` on a histogram family exposes the child with a sample count of zero and no bucket populated, and on a counter family exposes it at zero
- [ ] 4.7 `remove_label_values` removes only the named child, leaves its siblings exposed, and is a warning-and-no-op for a child that does not exist

## 5. Verification

- [ ] 5.1 `cargo test -p nym-metrics` green
- [ ] 5.2 `cargo check` clean for the only crates that can break, namely the two that construct metrics directly (`nym-node-metrics`, `nym-network-monitor-orchestrator`) plus one macro-only consumer to cover the `maybe_register_and_*` path
- [ ] 5.3 `cargo fmt` before committing, so the pre-commit hook does not reformat into the commit
