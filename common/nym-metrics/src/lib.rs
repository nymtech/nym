use dashmap::DashMap;
use std::fmt;
use tracing::{debug, error, warn};

use prometheus::{
    Encoder as _, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
    core::{Collector, MetricVec, MetricVecBuilder},
};

pub use prometheus::HistogramTimer;
pub use std::time::Instant;

#[macro_export]
macro_rules! prepend_package_name {
    ($name: tt) => {
        &format!(
            "{}_{}",
            std::module_path!()
                .split("::")
                .next()
                .unwrap_or("x")
                .to_string(),
            $name
        )
    };
}

#[macro_export]
macro_rules! inc_by {
    ($name:literal, $x:expr, $help: expr) => {
        $crate::REGISTRY.maybe_register_and_inc_by(
            $crate::prepend_package_name!($name),
            $x as i64,
            $help,
        );
    };
    ($name:literal, $x:expr) => {
        $crate::REGISTRY.maybe_register_and_inc_by(
            $crate::prepend_package_name!($name),
            $x as i64,
            None,
        );
    };
}

#[macro_export]
macro_rules! inc {
    ($name:literal, $help: expr) => {
        $crate::REGISTRY.maybe_register_and_inc($crate::prepend_package_name!($name), $help);
    };
    ($name:literal) => {
        $crate::REGISTRY.maybe_register_and_inc($crate::prepend_package_name!($name), None);
    };
}

#[macro_export]
macro_rules! metrics {
    () => {
        $crate::REGISTRY.to_string()
    };
}

#[macro_export]
macro_rules! set_metric {
    ($name:literal, $x:expr, $help: expr) => {
        $crate::REGISTRY.maybe_register_and_set(
            $crate::prepend_package_name!($name),
            $x as i64,
            $help,
        );
    };
    ($name:literal, $x:expr) => {
        $crate::REGISTRY.maybe_register_and_set(
            $crate::prepend_package_name!($name),
            $x as i64,
            None,
        );
    };
}

#[macro_export]
macro_rules! set_metric_float {
    ($name:literal, $x:expr, $help: expr) => {
        $crate::REGISTRY.maybe_register_and_set_float(
            $crate::prepend_package_name!($name),
            $x as f64,
            $help,
        );
    };
    ($name:literal, $x:expr) => {
        $crate::REGISTRY.maybe_register_and_set_float(
            $crate::prepend_package_name!($name),
            $x as f64,
            None,
        );
    };
}

#[macro_export]
macro_rules! add_histogram_obs {
    ($name:expr, $x:expr, $b:expr, $help:expr) => {
        $crate::REGISTRY.maybe_register_and_add_to_histogram(
            $crate::prepend_package_name!($name),
            $x as f64,
            Some($b),
            $help,
        );
    };

    ($name:expr, $x:expr, $b:expr) => {
        $crate::REGISTRY.maybe_register_and_add_to_histogram(
            $crate::prepend_package_name!($name),
            $x as f64,
            Some($b),
            None,
        );
    };
    ($name:expr, $x:expr) => {
        $crate::REGISTRY.maybe_register_and_add_to_histogram(
            $crate::prepend_package_name!($name),
            $x as f64,
            None,
            None,
        );
    };
}

#[macro_export]
macro_rules! nanos {
    ( $name:literal, $x:expr ) => {{
        let start = $crate::Instant::now();
        // if the block needs to return something, we can return it
        let r = $x;
        let duration = start.elapsed().as_nanos() as i64;
        let name = $crate::prepend_package_name!($name);
        $crate::REGISTRY.maybe_register_and_inc_by(&format!("{}_nanos", $name), duration, None);
        r
    }};
}

lazy_static::lazy_static! {
    pub static ref REGISTRY: MetricsController = MetricsController::default();
}

pub fn metrics_registry() -> &'static MetricsController {
    &REGISTRY
}

#[derive(Default)]
pub struct MetricsController {
    registry: Registry,
    registry_index: DashMap<String, Metric>,
}

pub enum Metric {
    IntCounter(Box<IntCounter>),
    IntGauge(Box<IntGauge>),
    FloatGauge(Box<Gauge>),
    Histogram(Box<Histogram>),
    IntCounterVec(Box<IntCounterVec>),
    IntGaugeVec(Box<IntGaugeVec>),
    FloatGaugeVec(Box<GaugeVec>),
    HistogramVec(Box<HistogramVec>),
}

impl Metric {
    pub fn new_int_counter(name: &str, help: &str) -> Option<Self> {
        match IntCounter::new(sanitize_metric_name(name), help) {
            Ok(c) => Some(c.into()),
            Err(err) => {
                error!("Failed to create counter {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_int_counter_with_labels(name: &str, help: &str, labels: &[&str]) -> Option<Self> {
        match IntCounterVec::new(Opts::new(sanitize_metric_name(name), help), labels) {
            Ok(c) => Some(c.into()),
            Err(err) => {
                error!("Failed to create counter family {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_int_gauge(name: &str, help: &str) -> Option<Self> {
        match IntGauge::new(sanitize_metric_name(name), help) {
            Ok(g) => Some(g.into()),
            Err(err) => {
                error!("Failed to create gauge {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_int_gauge_with_labels(name: &str, help: &str, labels: &[&str]) -> Option<Self> {
        match IntGaugeVec::new(Opts::new(sanitize_metric_name(name), help), labels) {
            Ok(g) => Some(g.into()),
            Err(err) => {
                error!("Failed to create gauge family {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_float_gauge(name: &str, help: &str) -> Option<Self> {
        match Gauge::new(sanitize_metric_name(name), help) {
            Ok(g) => Some(g.into()),
            Err(err) => {
                error!("Failed to create gauge {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_float_gauge_with_labels(name: &str, help: &str, labels: &[&str]) -> Option<Self> {
        match GaugeVec::new(Opts::new(sanitize_metric_name(name), help), labels) {
            Ok(g) => Some(g.into()),
            Err(err) => {
                error!("Failed to create gauge family {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_histogram(name: &str, help: &str, buckets: Option<&[f64]>) -> Option<Self> {
        match Histogram::with_opts(histogram_opts(name, help, buckets)) {
            Ok(h) => Some(h.into()),
            Err(err) => {
                error!("failed to create histogram {name:?}: {err}");
                None
            }
        }
    }

    pub fn new_histogram_with_labels(
        name: &str,
        help: &str,
        buckets: Option<&[f64]>,
        labels: &[&str],
    ) -> Option<Self> {
        match HistogramVec::new(histogram_opts(name, help, buckets), labels) {
            Ok(h) => Some(h.into()),
            Err(err) => {
                error!("failed to create histogram family {name:?}: {err}");
                None
            }
        }
    }

    fn as_collector(&self) -> Box<dyn Collector> {
        match self {
            Metric::IntCounter(c) => c.clone(),
            Metric::IntGauge(g) => g.clone(),
            Metric::FloatGauge(g) => g.clone(),
            Metric::Histogram(h) => h.clone(),
            Metric::IntCounterVec(c) => c.clone(),
            Metric::IntGaugeVec(g) => g.clone(),
            Metric::FloatGaugeVec(g) => g.clone(),
            Metric::HistogramVec(h) => h.clone(),
        }
    }
}

fn histogram_opts(name: &str, help: &str, buckets: Option<&[f64]>) -> HistogramOpts {
    let mut opts = HistogramOpts::new(sanitize_metric_name(name), help);
    if let Some(buckets) = buckets {
        opts = opts.buckets(buckets.to_vec())
    }
    opts
}

impl From<IntCounter> for Metric {
    fn from(v: IntCounter) -> Self {
        Metric::IntCounter(Box::new(v))
    }
}

impl From<IntGauge> for Metric {
    fn from(v: IntGauge) -> Self {
        Metric::IntGauge(Box::new(v))
    }
}

impl From<Gauge> for Metric {
    fn from(v: Gauge) -> Self {
        Metric::FloatGauge(Box::new(v))
    }
}

impl From<Histogram> for Metric {
    fn from(v: Histogram) -> Self {
        Metric::Histogram(Box::new(v))
    }
}

impl From<IntCounterVec> for Metric {
    fn from(v: IntCounterVec) -> Self {
        Metric::IntCounterVec(Box::new(v))
    }
}

impl From<IntGaugeVec> for Metric {
    fn from(v: IntGaugeVec) -> Self {
        Metric::IntGaugeVec(Box::new(v))
    }
}

impl From<GaugeVec> for Metric {
    fn from(v: GaugeVec) -> Self {
        Metric::FloatGaugeVec(Box::new(v))
    }
}

impl From<HistogramVec> for Metric {
    fn from(v: HistogramVec) -> Self {
        Metric::HistogramVec(Box::new(v))
    }
}

fn fq_name(c: &dyn Collector) -> String {
    c.desc()
        .first()
        .map(|d| d.fq_name.clone())
        .unwrap_or_default()
}

/// Resolves the child series addressed by `labels`, warning and yielding nothing on arity mismatch.
#[inline(always)]
fn child<T: MetricVecBuilder>(family: &MetricVec<T>, labels: &[&str]) -> Option<T::M> {
    match family.get_metric_with_label_values(labels) {
        Ok(series) => Some(series),
        Err(err) => {
            warn!(
                "could not resolve the child of '{}' addressed by {labels:?}: {err}",
                fq_name(family)
            );
            None
        }
    }
}

impl Metric {
    #[inline(always)]
    fn fq_name(&self) -> String {
        match self {
            Metric::IntCounter(c) => fq_name(c.as_ref()),
            Metric::IntGauge(g) => fq_name(g.as_ref()),
            Metric::FloatGauge(g) => fq_name(g.as_ref()),
            Metric::Histogram(h) => fq_name(h.as_ref()),
            Metric::IntCounterVec(c) => fq_name(c.as_ref()),
            Metric::IntGaugeVec(g) => fq_name(g.as_ref()),
            Metric::FloatGaugeVec(g) => fq_name(g.as_ref()),
            Metric::HistogramVec(h) => fq_name(h.as_ref()),
        }
    }

    /// Whether label values were supplied, which an unlabelled metric cannot accept.
    #[inline(always)]
    fn takes_labels(&self, labels: &[&str]) -> bool {
        if labels.is_empty() {
            return false;
        }
        warn!(
            "'{}' is not a labelled family, but {labels:?} were supplied",
            self.fq_name()
        );
        true
    }

    #[inline(always)]
    fn inc(&self, labels: &[&str]) {
        match self {
            Metric::IntCounter(c) => {
                if !self.takes_labels(labels) {
                    c.inc()
                }
            }
            Metric::IntGauge(g) => {
                if !self.takes_labels(labels) {
                    g.inc()
                }
            }
            Metric::FloatGauge(g) => {
                if !self.takes_labels(labels) {
                    g.inc()
                }
            }
            Metric::IntCounterVec(c) => {
                if let Some(series) = child(c.as_ref(), labels) {
                    series.inc()
                }
            }
            Metric::IntGaugeVec(g) => {
                if let Some(series) = child(g.as_ref(), labels) {
                    series.inc()
                }
            }
            Metric::FloatGaugeVec(g) => {
                if let Some(series) = child(g.as_ref(), labels) {
                    series.inc()
                }
            }
            Metric::Histogram(_) | Metric::HistogramVec(_) => {
                warn!("invalid operation: attempted to call increment on a histogram")
            }
        }
    }

    #[inline(always)]
    fn inc_by(&self, value: i64, labels: &[&str]) {
        match self {
            Metric::IntCounter(c) => {
                if !self.takes_labels(labels) {
                    c.inc_by(value as u64)
                }
            }
            Metric::IntGauge(g) => {
                if !self.takes_labels(labels) {
                    g.add(value)
                }
            }
            Metric::FloatGauge(g) => {
                warn!(
                    "attempted to increment a float gauge ('{}') by an integer - this is most likely a bug",
                    self.fq_name()
                );
                if !self.takes_labels(labels) {
                    g.add(value as f64)
                }
            }
            Metric::IntCounterVec(c) => {
                if let Some(series) = child(c.as_ref(), labels) {
                    series.inc_by(value as u64)
                }
            }
            Metric::IntGaugeVec(g) => {
                if let Some(series) = child(g.as_ref(), labels) {
                    series.add(value)
                }
            }
            Metric::FloatGaugeVec(g) => {
                warn!(
                    "attempted to increment a float gauge ('{}') by an integer - this is most likely a bug",
                    self.fq_name()
                );
                if let Some(series) = child(g.as_ref(), labels) {
                    series.add(value as f64)
                }
            }
            Metric::Histogram(_) | Metric::HistogramVec(_) => {
                warn!("invalid operation: attempted to call increment on a histogram")
            }
        }
    }

    #[inline(always)]
    fn set(&self, value: i64, labels: &[&str]) {
        match self {
            Metric::IntCounter(_) | Metric::IntCounterVec(_) => {
                warn!("Cannot set value for counter {:?}", self.fq_name());
            }
            Metric::IntGauge(g) => {
                if !self.takes_labels(labels) {
                    g.set(value)
                }
            }
            Metric::FloatGauge(g) => {
                warn!(
                    "attempted to set a float gauge ('{}') to an integer value - this is most likely a bug",
                    self.fq_name()
                );
                if !self.takes_labels(labels) {
                    g.set(value as f64)
                }
            }
            Metric::IntGaugeVec(g) => {
                if let Some(series) = child(g.as_ref(), labels) {
                    series.set(value)
                }
            }
            Metric::FloatGaugeVec(g) => {
                warn!(
                    "attempted to set a float gauge ('{}') to an integer value - this is most likely a bug",
                    self.fq_name()
                );
                if let Some(series) = child(g.as_ref(), labels) {
                    series.set(value as f64)
                }
            }
            Metric::Histogram(_) | Metric::HistogramVec(_) => {
                warn!("invalid operation: attempted to call set on a histogram")
            }
        }
    }

    #[inline(always)]
    fn set_float(&self, value: f64, labels: &[&str]) {
        match self {
            Metric::IntCounter(_) | Metric::IntCounterVec(_) => {
                warn!("Cannot set value for counter {:?}", self.fq_name());
            }
            Metric::IntGauge(g) => {
                warn!(
                    "attempted to set a integer gauge ('{}') to a float value - this is most likely a bug",
                    self.fq_name()
                );
                if !self.takes_labels(labels) {
                    g.set(value as i64)
                }
            }
            Metric::FloatGauge(g) => {
                if !self.takes_labels(labels) {
                    g.set(value)
                }
            }
            Metric::IntGaugeVec(g) => {
                warn!(
                    "attempted to set a integer gauge ('{}') to a float value - this is most likely a bug",
                    self.fq_name()
                );
                if let Some(series) = child(g.as_ref(), labels) {
                    series.set(value as i64)
                }
            }
            Metric::FloatGaugeVec(g) => {
                if let Some(series) = child(g.as_ref(), labels) {
                    series.set(value)
                }
            }
            Metric::Histogram(_) | Metric::HistogramVec(_) => {
                warn!("invalid operation: attempted to call increment on a histogram")
            }
        }
    }

    #[inline(always)]
    fn add_histogram_observation(&self, value: f64, labels: &[&str]) {
        match self {
            Metric::Histogram(h) => {
                if !self.takes_labels(labels) {
                    h.observe(value)
                }
            }
            Metric::HistogramVec(h) => {
                if let Some(series) = child(h.as_ref(), labels) {
                    series.observe(value)
                }
            }
            _ => warn!("attempted to add histogram observation on a non-histogram metric"),
        }
    }

    #[inline(always)]
    fn start_timer(&self, labels: &[&str]) -> Option<HistogramTimer> {
        match self {
            Metric::Histogram(h) => (!self.takes_labels(labels)).then(|| h.start_timer()),
            Metric::HistogramVec(h) => child(h.as_ref(), labels).map(|series| series.start_timer()),
            _ => {
                warn!("attempted to start histogram observation on a non-histogram metric");
                None
            }
        }
    }

    #[inline(always)]
    fn init(&self, labels: &[&str]) -> bool {
        match self {
            Metric::IntCounterVec(c) => child(c.as_ref(), labels).is_some(),
            Metric::IntGaugeVec(g) => child(g.as_ref(), labels).is_some(),
            Metric::FloatGaugeVec(g) => child(g.as_ref(), labels).is_some(),
            Metric::HistogramVec(h) => child(h.as_ref(), labels).is_some(),
            _ => {
                warn!(
                    "'{}' is not a labelled family, so it has no child series to create",
                    self.fq_name()
                );
                false
            }
        }
    }

    #[inline(always)]
    fn remove(&self, labels: &[&str]) -> bool {
        let removed = match self {
            Metric::IntCounterVec(c) => c.remove_label_values(labels),
            Metric::IntGaugeVec(g) => g.remove_label_values(labels),
            Metric::FloatGaugeVec(g) => g.remove_label_values(labels),
            Metric::HistogramVec(h) => h.remove_label_values(labels),
            _ => {
                warn!(
                    "'{}' is not a labelled family, so it has no child series to remove",
                    self.fq_name()
                );
                return false;
            }
        };

        match removed {
            Ok(_) => true,
            Err(err) => {
                warn!(
                    "could not remove the child of '{}' addressed by {labels:?}: {err}",
                    self.fq_name()
                );
                false
            }
        }
    }
}

impl fmt::Display for MetricsController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metrics = self.gather();
        let output = match String::from_utf8(metrics) {
            Ok(output) => output,
            Err(e) => return write!(f, "Error decoding metrics to String: {e}"),
        };
        write!(f, "{output}")
    }
}

impl MetricsController {
    #[inline(always)]
    pub fn gather(&self) -> Vec<u8> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metrics = self.registry.gather();
        match encoder.encode(&metrics, &mut buffer) {
            Ok(_) => {}
            Err(e) => error!("Error encoding metrics to buffer: {}", e),
        }
        buffer
    }

    #[inline(always)]
    pub fn to_writer(&self, writer: &mut dyn std::io::Write) {
        let metrics = self.gather();
        match writer.write_all(&metrics) {
            Ok(_) => {}
            Err(e) => error!("Error writing metrics to writer: {}", e),
        }
    }

    #[inline(always)]
    pub fn register_int_gauge<'a>(&self, name: &str, help: impl Into<Option<&'a str>>) {
        let Some(metric) = Metric::new_int_gauge(name, help.into().unwrap_or(name)) else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_int_gauge_with_labels<'a>(
        &self,
        name: &str,
        help: impl Into<Option<&'a str>>,
        labels: &[&str],
    ) {
        let Some(metric) =
            Metric::new_int_gauge_with_labels(name, help.into().unwrap_or(name), labels)
        else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_float_gauge<'a>(&self, name: &str, help: impl Into<Option<&'a str>>) {
        let Some(metric) = Metric::new_float_gauge(name, help.into().unwrap_or(name)) else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_float_gauge_with_labels<'a>(
        &self,
        name: &str,
        help: impl Into<Option<&'a str>>,
        labels: &[&str],
    ) {
        let Some(metric) =
            Metric::new_float_gauge_with_labels(name, help.into().unwrap_or(name), labels)
        else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_int_counter<'a>(&self, name: &str, help: impl Into<Option<&'a str>>) {
        let Some(metric) = Metric::new_int_counter(name, help.into().unwrap_or(name)) else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_int_counter_with_labels<'a>(
        &self,
        name: &str,
        help: impl Into<Option<&'a str>>,
        labels: &[&str],
    ) {
        let Some(metric) =
            Metric::new_int_counter_with_labels(name, help.into().unwrap_or(name), labels)
        else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_histogram<'a>(
        &self,
        name: &str,
        help: impl Into<Option<&'a str>>,
        buckets: Option<&[f64]>,
    ) {
        let Some(metric) = Metric::new_histogram(name, help.into().unwrap_or(name), buckets) else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn register_histogram_with_labels<'a>(
        &self,
        name: &str,
        help: impl Into<Option<&'a str>>,
        buckets: Option<&[f64]>,
        labels: &[&str],
    ) {
        let Some(metric) =
            Metric::new_histogram_with_labels(name, help.into().unwrap_or(name), buckets, labels)
        else {
            return;
        };
        self.register_metric(metric);
    }

    #[inline(always)]
    pub fn set(&self, name: &str, value: i64) -> bool {
        self.set_with_labels(name, value, &[])
    }

    #[inline(always)]
    pub fn set_with_labels(&self, name: &str, value: i64, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set(value, labels);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn set_float(&self, name: &str, value: f64) -> bool {
        self.set_float_with_labels(name, value, &[])
    }

    #[inline(always)]
    pub fn set_float_with_labels(&self, name: &str, value: f64, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set_float(value, labels);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn add_to_histogram(&self, name: &str, value: f64) -> bool {
        self.add_to_histogram_with_labels(name, value, &[])
    }

    #[inline(always)]
    pub fn add_to_histogram_with_labels(&self, name: &str, value: f64, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.add_histogram_observation(value, labels);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn start_timer(&self, name: &str) -> Option<HistogramTimer> {
        self.start_timer_with_labels(name, &[])
    }

    #[inline(always)]
    pub fn start_timer_with_labels(&self, name: &str, labels: &[&str]) -> Option<HistogramTimer> {
        self.registry_index
            .get(name)
            .and_then(|metric| metric.start_timer(labels))
    }

    #[inline(always)]
    pub fn inc(&self, name: &str) -> bool {
        self.inc_with_labels(name, &[])
    }

    #[inline(always)]
    pub fn inc_with_labels(&self, name: &str, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc(labels);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn inc_by(&self, name: &str, value: i64) -> bool {
        self.inc_by_with_labels(name, value, &[])
    }

    #[inline(always)]
    pub fn inc_by_with_labels(&self, name: &str, value: i64, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc_by(value, labels);
            true
        } else {
            false
        }
    }

    /// Creates the child series addressed by `labels` so it is exposed, zeroed, from the next scrape.
    ///
    /// The registry bounds nothing: a consumer whose label values churn owns removing its dead children.
    #[inline(always)]
    pub fn init_label_values(&self, name: &str, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.init(labels)
        } else {
            false
        }
    }

    /// Removes the child series addressed by `labels`, leaving the family's other children exposed.
    ///
    /// The registry bounds nothing: a consumer whose label values churn owns removing its dead children.
    #[inline(always)]
    pub fn remove_label_values(&self, name: &str, labels: &[&str]) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.remove(labels)
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn maybe_register_and_set<'a>(
        &self,
        name: &str,
        value: i64,
        help: impl Into<Option<&'a str>>,
    ) {
        if !self.set(name, value) {
            let help = help.into();
            self.register_int_gauge(name, help);
            self.set(name, value);
        }
    }

    #[inline(always)]
    pub fn maybe_register_and_set_float<'a>(
        &self,
        name: &str,
        value: f64,
        help: impl Into<Option<&'a str>>,
    ) {
        if !self.set_float(name, value) {
            let help = help.into();
            self.register_float_gauge(name, help);
            self.set_float(name, value);
        }
    }

    #[inline(always)]
    pub fn maybe_register_and_add_to_histogram<'a>(
        &self,
        name: &str,
        value: f64,
        buckets: Option<&[f64]>,
        help: impl Into<Option<&'a str>>,
    ) {
        if !self.add_to_histogram(name, value) {
            let help = help.into();
            self.register_histogram(name, help, buckets);
            self.add_to_histogram(name, value);
        }
    }

    #[inline(always)]
    pub fn maybe_register_and_inc<'a>(&self, name: &str, help: impl Into<Option<&'a str>>) {
        if !self.inc(name) {
            let help = help.into();
            self.register_int_counter(name, help);
            self.inc(name);
        }
    }

    #[inline(always)]
    pub fn maybe_register_and_inc_by<'a>(
        &self,
        name: &str,
        value: i64,
        help: impl Into<Option<&'a str>>,
    ) {
        if !self.inc_by(name, value) {
            let help = help.into();
            self.register_int_counter(name, help);
            self.inc_by(name, value);
        }
    }

    #[inline(always)]
    pub fn register_metric(&self, metric: impl Into<Metric>) {
        let m = metric.into();
        let fq_name = m.fq_name();

        if self.registry_index.contains_key(&fq_name) {
            return;
        }

        match self.registry.register(m.as_collector()) {
            Ok(_) => {
                self.registry_index.insert(fq_name, m);
            }
            Err(err) => {
                debug!("Failed to register '{fq_name}': {err}")
            }
        }
    }
}

fn sanitize_metric_name(name: &str) -> String {
    // The first character must be [a-zA-Z_:], and all subsequent characters must be [a-zA-Z0-9_:].
    let mut out = String::with_capacity(name.len());
    let mut is_invalid: fn(char) -> bool = invalid_metric_name_start_character;
    for c in name.chars() {
        if is_invalid(c) {
            out.push('_');
        } else {
            out.push(c);
        }
        is_invalid = invalid_metric_name_character;
    }
    out
}

#[inline]
fn invalid_metric_name_start_character(c: char) -> bool {
    // Essentially, needs to match the regex pattern of [a-zA-Z_:].
    !(c.is_ascii_alphabetic() || c == '_' || c == ':')
}

#[inline]
fn invalid_metric_name_character(c: char) -> bool {
    // Essentially, needs to match the regex pattern of [a-zA-Z0-9_:].
    !(c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUCKETS: &[f64] = &[0.5, 1.0, 2.5];

    fn exposition(controller: &MetricsController) -> String {
        String::from_utf8(controller.gather()).expect("the exposition is not valid utf-8")
    }

    #[test]
    fn test_sanitization() {
        assert_eq!(
            sanitize_metric_name("packets_sent_34.242.65.133:1789"),
            "packets_sent_34_242_65_133:1789"
        )
    }

    #[test]
    fn prepend_package_name() {
        let literal = prepend_package_name!("foo");
        assert_eq!(literal, "nym_metrics_foo");

        let bar = "bar";
        let format = format!("foomp_{bar}");
        let formatted = prepend_package_name!(format);
        assert_eq!(formatted, "nym_metrics_foomp_bar");
    }

    #[test]
    fn unlabelled_metrics_render_without_a_label_set() {
        let controller = MetricsController::default();
        controller.register_int_counter("counter", "a counter");
        controller.register_int_gauge("int_gauge", "an int gauge");
        controller.register_float_gauge("float_gauge", "a float gauge");
        controller.register_histogram("histogram", "a histogram", Some(BUCKETS));

        assert!(controller.inc_by("counter", 7));
        assert!(controller.set("int_gauge", -3));
        assert!(controller.set_float("float_gauge", 1.5));
        assert!(controller.add_to_histogram("histogram", 0.75));

        let rendered = exposition(&controller);
        assert!(rendered.contains("\ncounter 7\n"), "{rendered}");
        assert!(rendered.contains("\nint_gauge -3\n"), "{rendered}");
        assert!(rendered.contains("\nfloat_gauge 1.5\n"), "{rendered}");
        assert!(rendered.contains("\nhistogram_count 1\n"), "{rendered}");
        assert!(rendered.contains("\nhistogram_sum 0.75\n"), "{rendered}");
    }

    #[test]
    fn a_labelled_family_exposes_one_series_per_label_value() {
        let controller = MetricsController::default();
        controller.register_int_counter_with_labels("assignments", "assignments", &["kind"]);

        assert!(controller.inc_with_labels("assignments", &["liveness"]));
        assert!(controller.inc_by_with_labels("assignments", 2, &["stress"]));

        let rendered = exposition(&controller);
        assert_eq!(rendered.matches("# TYPE assignments counter").count(), 1);
        assert!(
            rendered.contains("assignments{kind=\"liveness\"} 1"),
            "{rendered}"
        );
        assert!(
            rendered.contains("assignments{kind=\"stress\"} 2"),
            "{rendered}"
        );
    }

    #[test]
    fn children_of_one_family_are_independent() {
        let controller = MetricsController::default();
        controller.register_int_gauge_with_labels("pending", "pending", &["kind"]);

        assert!(controller.set_with_labels("pending", 4, &["liveness"]));
        assert!(controller.set_with_labels("pending", 9, &["stress"]));
        assert!(controller.set_with_labels("pending", 1, &["liveness"]));

        let rendered = exposition(&controller);
        assert!(
            rendered.contains("pending{kind=\"liveness\"} 1"),
            "{rendered}"
        );
        assert!(
            rendered.contains("pending{kind=\"stress\"} 9"),
            "{rendered}"
        );
    }

    #[test]
    fn a_timer_observes_against_its_own_child() {
        let controller = MetricsController::default();
        controller.register_histogram_with_labels("duration", "duration", Some(BUCKETS), &["kind"]);
        controller.init_label_values("duration", &["stress"]);

        drop(controller.start_timer_with_labels("duration", &["liveness"]));

        let rendered = exposition(&controller);
        assert!(
            rendered.contains("duration_count{kind=\"liveness\"} 1"),
            "{rendered}"
        );
        assert!(
            rendered.contains("duration_count{kind=\"stress\"} 0"),
            "{rendered}"
        );
    }

    #[test]
    fn an_arity_mismatch_against_a_family_changes_nothing() {
        let controller = MetricsController::default();
        controller.register_int_gauge_with_labels("pending", "pending", &["kind"]);
        assert!(controller.set_with_labels("pending", 4, &["liveness"]));
        let before = exposition(&controller);

        // too many label values, and then too few
        assert!(controller.set_with_labels("pending", 9, &["liveness", "stress"]));
        assert!(controller.set("pending", 9));

        assert_eq!(exposition(&controller), before);
    }

    #[test]
    fn label_values_on_an_unlabelled_metric_change_nothing() {
        let controller = MetricsController::default();
        controller.register_int_gauge("pending", "pending");
        assert!(controller.set("pending", 4));
        let before = exposition(&controller);

        assert!(controller.set_with_labels("pending", 9, &["liveness"]));
        assert!(!controller.init_label_values("pending", &["liveness"]));
        assert!(!controller.remove_label_values("pending", &["liveness"]));

        assert_eq!(exposition(&controller), before);
    }

    #[test]
    fn recording_against_an_unregistered_name_does_not_apply() {
        let controller = MetricsController::default();

        assert!(!controller.inc("absent"));
        assert!(!controller.inc_by("absent", 1));
        assert!(!controller.set_with_labels("absent", 1, &["kind"]));
        assert!(controller.start_timer("absent").is_none());
        assert!(!controller.init_label_values("absent", &["kind"]));
        assert!(!controller.remove_label_values("absent", &["kind"]));
    }

    #[test]
    fn init_label_values_exposes_a_child_without_observing() {
        let controller = MetricsController::default();
        controller.register_histogram_with_labels("duration", "duration", Some(BUCKETS), &["kind"]);
        controller.register_int_counter_with_labels("errors", "errors", &["kind"]);

        assert!(controller.init_label_values("duration", &["liveness"]));
        assert!(controller.init_label_values("errors", &["liveness"]));

        let rendered = exposition(&controller);
        assert!(
            rendered.contains("duration_count{kind=\"liveness\"} 0"),
            "{rendered}"
        );
        assert!(
            rendered.contains("duration_sum{kind=\"liveness\"} 0"),
            "{rendered}"
        );
        assert!(
            rendered.contains("errors{kind=\"liveness\"} 0"),
            "{rendered}"
        );
        for bucket in rendered
            .lines()
            .filter(|line| line.starts_with("duration_bucket"))
        {
            assert!(bucket.ends_with(" 0"), "initialisation populated {bucket}");
        }
    }

    #[test]
    fn remove_label_values_removes_only_the_named_child() {
        let controller = MetricsController::default();
        controller.register_int_gauge_with_labels("pending", "pending", &["kind"]);
        assert!(controller.set_with_labels("pending", 1, &["liveness"]));
        assert!(controller.set_with_labels("pending", 2, &["stress"]));

        assert!(controller.remove_label_values("pending", &["liveness"]));

        let rendered = exposition(&controller);
        assert!(!rendered.contains("kind=\"liveness\""), "{rendered}");
        assert!(
            rendered.contains("pending{kind=\"stress\"} 2"),
            "{rendered}"
        );

        // removing it a second time is a warning and a no-op
        assert!(!controller.remove_label_values("pending", &["liveness"]));
        assert_eq!(exposition(&controller), rendered);
    }
}
