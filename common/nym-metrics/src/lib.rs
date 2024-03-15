pub use cfg_if;
use dashmap::DashMap;
use log::{debug, warn};
use regex::Regex;
use std::fmt;

use prometheus::{core::Collector, Counter, Encoder as _, Gauge, Registry, TextEncoder};

pub fn cpu_cycles() -> Result<i64, Box<dyn std::error::Error>> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "cpucycles")] {
            Ok(cpu_cycles::cpucycles()?)
        } else {
            Err("`cpucycles` feature is not turned on!".into())
        }
    }
}

#[macro_export]
macro_rules! measure {
    ( $name:expr, $x:expr ) => {{
        $crate::cfg_if::cfg_if! {
            if #[cfg(feature = "cpucycles")] {
                let start_cycles = $crate::cpu_cycles();
                // if the block needs to return something, we can return it
                let r = $x;
                let end_cycles = $crate::cpu_cycles();
                match (start_cycles, end_cycles) {
                    (Ok(start), Ok(end)) => {
                        $crate::REGISTRY.inc_by($name, (end - start) as f64);
                    },
                    (Err(e), _) => error!("{e}"),
                    (_, Err(e)) => error!("{e}"),
                }
                r
            } else {
                $x
            }
        }
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
    fn fq_name(&self) -> String {
        match self {
            Metric::C(c) => fq_name(c.as_ref()),
            Metric::G(g) => fq_name(g.as_ref()),
        }
    }

    fn inc_by(&self, value: f64) {
        match self {
            Metric::C(c) => c.inc_by(value),
            Metric::G(g) => g.add(value),
        }
    }

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
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metrics = self.registry.gather();
        match encoder.encode(&metrics, &mut buffer) {
            Ok(_) => {}
            Err(e) => return write!(f, "Error encoding metrics to buffer: {}", e),
        }
        let output = match String::from_utf8(buffer) {
            Ok(output) => output,
            Err(e) => return write!(f, "Error decoding metrics to String: {}", e),
        };
        write!(f, "{}", output)
    }
}

impl MetricsController {
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
