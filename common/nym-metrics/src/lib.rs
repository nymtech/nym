use dashmap::DashMap;
pub use log::error;
use log::{debug, warn};
use regex::Regex;
use std::fmt;
pub use std::time::Instant;

use prometheus::{core::Collector, Counter, Encoder as _, Gauge, Registry, TextEncoder};

#[macro_export]
macro_rules! inc_by {
    ($name:literal, $x:expr) => {
        $crate::REGISTRY.inc_by($name, $x as f64);
    };
}

#[macro_export]
macro_rules! inc {
    ($name:literal) => {
        $crate::REGISTRY.inc($name);
    };
}

#[macro_export]
macro_rules! metrics {
    () => {
        $crate::REGISTRY.to_string();
    };
}

#[macro_export]
macro_rules! nanos {
    ( $name:literal, $x:expr ) => {{
        let start = $crate::Instant::now();
        // if the block needs to return something, we can return it
        let r = $x;
        let duration = start.elapsed().as_nanos() as f64;
        $crate::REGISTRY.inc_by(&format!("{}_nanos", $name), duration);
        r
    }};
}

lazy_static::lazy_static! {
    pub static ref RE: Regex = Regex::new(r"[^a-zA-Z0-9_]").unwrap();
    pub static ref REGISTRY: MetricsController = MetricsController::default();
}

#[derive(Default)]
pub struct MetricsController {
    registry: Registry,
    registry_index: DashMap<String, Metric>,
}

enum Metric {
    C(Box<Counter>),
    G(Box<Gauge>),
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
            Metric::C(c) => fq_name(c.as_ref()),
            Metric::G(g) => fq_name(g.as_ref()),
        }
    }

    #[inline(always)]
    fn inc(&self) {
        match self {
            Metric::C(c) => c.inc(),
            Metric::G(g) => g.inc(),
        }
    }

    #[inline(always)]
    fn inc_by(&self, value: f64) {
        match self {
            Metric::C(c) => c.inc_by(value),
            Metric::G(g) => g.add(value),
        }
    }

    #[inline(always)]
    fn set(&self, value: f64) {
        match self {
            Metric::C(_c) => {
                warn!("Cannot set value for counter {:?}", self.fq_name());
            }
            Metric::G(g) => g.set(value),
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

    pub fn set(&self, name: &str, value: f64) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.set(value);
        } else {
            let gauge = match Gauge::new(sanitize_metric_name(name), name) {
                Ok(g) => g,
                Err(e) => {
                    debug!("Failed to create gauge {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_gauge(Box::new(gauge));
            self.set(name, value)
        }
    }

    pub fn inc(&self, name: &str) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc();
        } else {
            let counter = match Counter::new(sanitize_metric_name(name), name) {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to create counter {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_counter(Box::new(counter));
            self.inc(name)
        }
    }

    pub fn inc_by(&self, name: &str, value: f64) {
        if let Some(metric) = self.registry_index.get(name) {
            metric.inc_by(value);
        } else {
            let counter = match Counter::new(sanitize_metric_name(name), name) {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to create counter {:?}:\n{}", name, e);
                    return;
                }
            };
            self.register_counter(Box::new(counter));
            self.inc_by(name, value)
        }
    }

    fn register_gauge(&self, metric: Box<Gauge>) {
        let fq_name = metric
            .desc()
            .first()
            .map(|d| d.fq_name.clone())
            .unwrap_or_default();

        if self.registry_index.contains_key(&fq_name) {
            return;
        }

        match self.registry.register(metric.clone()) {
            Ok(_) => {
                self.registry_index
                    .insert(fq_name, Metric::G(metric.clone()));
            }
            Err(e) => {
                debug!("Failed to register {:?}:\n{}", fq_name, e)
            }
        }
    }

    fn register_counter(&self, metric: Box<Counter>) {
        let fq_name = metric
            .desc()
            .first()
            .map(|d| d.fq_name.clone())
            .unwrap_or_default();

        if self.registry_index.contains_key(&fq_name) {
            return;
        }
        match self.registry.register(metric.clone()) {
            Ok(_) => {
                self.registry_index
                    .insert(fq_name, Metric::C(metric.clone()));
            }
            Err(e) => {
                debug!("Failed to register {:?}:\n{}", fq_name, e)
            }
        }
    }
}

#[inline(always)]
fn sanitize_metric_name(name: &str) -> String {
    RE.replace_all(name, "_").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitization() {
        assert_eq!(
            sanitize_metric_name("packets_sent_34.242.65.133:1789"),
            "packets_sent_34_242_65_133_1789"
        )
    }
}
