use dashmap::DashMap;
use std::fmt;
use tracing::{debug, error, warn};

use prometheus::{
    core::Collector, Encoder as _, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Registry,
    TextEncoder,
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
        $crate::REGISTRY.to_string();
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

    pub fn new_int_gauge(name: &str, help: &str) -> Option<Self> {
        match IntGauge::new(sanitize_metric_name(name), help) {
            Ok(g) => Some(g.into()),
            Err(err) => {
                error!("Failed to create gauge {name:?}: {err}");
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

    pub fn new_histogram(name: &str, help: &str, buckets: Option<&[f64]>) -> Option<Self> {
        let mut opts = HistogramOpts::new(sanitize_metric_name(name), help);
        if let Some(buckets) = buckets {
            opts = opts.buckets(buckets.to_vec())
        }
        match Histogram::with_opts(opts) {
            Ok(h) => Some(Metric::Histogram(Box::new(h))),
            Err(err) => {
                error!("failed to create histogram {name:?}: {err}");
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
        }
    }
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

fn fq_name(c: &dyn Collector) -> String {
    c.desc()
        .first()
        .map(|d| d.fq_name.clone())
        .unwrap_or_default()
}

impl Metric {
    #[inline(always)]
    fn fq_name(&self) -> String {
        match self {
            Metric::IntCounter(c) => fq_name(c.as_ref()),
            Metric::IntGauge(g) => fq_name(g.as_ref()),
            Metric::FloatGauge(g) => fq_name(g.as_ref()),
            Metric::Histogram(h) => fq_name(h.as_ref()),
        }
    }

    #[inline(always)]
    fn inc(&self) {
        match self {
            Metric::IntCounter(c) => c.inc(),
            Metric::IntGauge(g) => g.inc(),
            Metric::FloatGauge(g) => g.inc(),
            Metric::Histogram(_) => {
                warn!("invalid operation: attempted to call increment on a histogram")
            }
        }
    }

    #[inline(always)]
    fn inc_by(&self, value: i64) {
        match self {
            Metric::IntCounter(c) => c.inc_by(value as u64),
            Metric::IntGauge(g) => g.add(value),
            Metric::FloatGauge(g) => {
                warn!("attempted to increment a float gauge ('{}') by an integer - this is most likely a bug", self.fq_name());
                g.add(value as f64)
            }
            Metric::Histogram(_) => {
                warn!("invalid operation: attempted to call increment on a histogram")
            }
        }
    }

    #[inline(always)]
    fn set(&self, value: i64) {
        match self {
            Metric::IntCounter(_c) => {
                warn!("Cannot set value for counter {:?}", self.fq_name());
            }
            Metric::IntGauge(g) => g.set(value),
            Metric::FloatGauge(g) => {
                warn!("attempted to set a float gauge ('{}') to an integer value - this is most likely a bug", self.fq_name());
                g.set(value as f64)
            }
            Metric::Histogram(_) => {
                warn!("invalid operation: attempted to call set on a histogram")
            }
        }
    }

    #[inline(always)]
    fn set_float(&self, value: f64) {
        match self {
            Metric::IntCounter(_c) => {
                warn!("Cannot set value for counter {:?}", self.fq_name());
            }
            Metric::IntGauge(g) => {
                warn!("attempted to set a integer gauge ('{}') to a float value - this is most likely a bug", self.fq_name());
                g.set(value as i64)
            }
            Metric::FloatGauge(g) => g.set(value),
            Metric::Histogram(_) => {
                warn!("invalid operation: attempted to call increment on a histogram")
            }
        }
    }

    #[inline(always)]
    fn add_histogram_observation(&self, value: f64) {
        match self {
            Metric::Histogram(h) => {
                h.observe(value);
            }
            _ => warn!("attempted to add histogram observation on a non-histogram metric"),
        }
    }

    #[inline(always)]
    fn start_timer(&self) -> Option<HistogramTimer> {
        match self {
            Metric::Histogram(h) => Some(h.start_timer()),
            _ => {
                warn!("attempted to start histogram observation on a non-histogram metric");
                None
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
    pub fn register_float_gauge<'a>(&self, name: &str, help: impl Into<Option<&'a str>>) {
        let Some(metric) = Metric::new_float_gauge(name, help.into().unwrap_or(name)) else {
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
    pub fn set(&self, name: &str, value: i64) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set(value);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn set_float(&self, name: &str, value: f64) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set_float(value);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn add_to_histogram(&self, name: &str, value: f64) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.add_histogram_observation(value);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn start_timer(&self, name: &str) -> Option<HistogramTimer> {
        self.registry_index
            .get(name)
            .and_then(|metric| metric.start_timer())
    }

    #[inline(always)]
    pub fn inc(&self, name: &str) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc();
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn inc_by(&self, name: &str, value: i64) -> bool {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc_by(value);
            true
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
}
