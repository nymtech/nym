use dashmap::DashMap;
pub use log::error;
use log::{debug, warn};
use std::fmt;
pub use std::time::Instant;

use prometheus::{
    core::Collector, Encoder as _, Gauge, IntCounter, IntGauge, Registry, TextEncoder,
};

#[macro_export]
macro_rules! prepend_package_name {
    ($name: literal) => {
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
    ($name:literal, $x:expr) => {
        $crate::REGISTRY.inc_by($crate::prepend_package_name!($name), $x as i64);
    };
}

#[macro_export]
macro_rules! inc {
    ($name:literal) => {
        $crate::REGISTRY.inc($crate::prepend_package_name!($name));
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
    ($name:literal, $x:expr) => {
        $crate::REGISTRY.set($crate::prepend_package_name!($name), $x as i64);
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
        $crate::REGISTRY.inc_by(&format!("{}_nanos", $name), duration);
        r
    }};
}

lazy_static::lazy_static! {
    pub static ref REGISTRY: MetricsController = MetricsController::default();
}

#[derive(Default)]
pub struct MetricsController {
    registry: Registry,
    registry_index: DashMap<String, Metric>,
}

enum Metric {
    IntCounter(Box<IntCounter>),
    IntGauge(Box<IntGauge>),
    FloatGauge(Box<Gauge>),
}

impl Metric {
    fn as_collector(&self) -> Box<dyn Collector> {
        match self {
            Metric::IntCounter(c) => c.clone(),
            Metric::IntGauge(g) => g.clone(),
            Metric::FloatGauge(g) => g.clone(),
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
        }
    }

    #[inline(always)]
    fn inc(&self) {
        match self {
            Metric::IntCounter(c) => c.inc(),
            Metric::IntGauge(g) => g.inc(),
            Metric::FloatGauge(g) => g.inc(),
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
        }
    }
}

impl fmt::Display for MetricsController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metrics = self.gather();
        let output = match String::from_utf8(metrics) {
            Ok(output) => output,
            Err(e) => return write!(f, "Error decoding metrics to String: {}", e),
        };
        write!(f, "{}", output)
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

    pub fn to_writer(&self, writer: &mut dyn std::io::Write) {
        let metrics = self.gather();
        match writer.write_all(&metrics) {
            Ok(_) => {}
            Err(e) => error!("Error writing metrics to writer: {}", e),
        }
    }

    pub fn set(&self, name: &str, value: i64) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set(value);
        } else {
            let gauge = match IntGauge::new(sanitize_metric_name(name), name) {
                Ok(g) => g,
                Err(e) => {
                    debug!("Failed to create gauge {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_metric(gauge);
            self.set(name, value)
        }
    }

    pub fn set_float(&self, name: &str, value: f64) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set_float(value);
        } else {
            let gauge = match Gauge::new(sanitize_metric_name(name), name) {
                Ok(g) => g,
                Err(e) => {
                    debug!("Failed to create gauge {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_metric(gauge);
            self.set_float(name, value)
        }
    }

    pub fn inc(&self, name: &str) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc();
        } else {
            let counter = match IntCounter::new(sanitize_metric_name(name), name) {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to create counter {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_metric(counter);
            self.inc(name)
        }
    }

    pub fn inc_by(&self, name: &str, value: i64) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc_by(value);
        } else {
            let counter = match IntCounter::new(sanitize_metric_name(name), name) {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to create counter {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_metric(counter);
            self.inc_by(name, value)
        }
    }

    fn register_metric(&self, metric: impl Into<Metric>) {
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
}
