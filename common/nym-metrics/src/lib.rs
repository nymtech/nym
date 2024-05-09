use dashmap::DashMap;
pub use log::error;
use log::{debug, warn};
use std::fmt;
pub use std::time::Instant;

use prometheus::{core::Collector, Encoder as _, IntCounter, IntGauge, Registry, TextEncoder};

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
macro_rules! now {
    () => {
        match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => 0,
        }
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
macro_rules! fragment_recieved {
    ($seed:expr) => {
        $crate::FRAGMENTS_RECEIVED.insert($seed, $crate::now!());
    };
}

#[macro_export]
macro_rules! fragment_sent {
    ($seed:expr) => {
        $crate::FRAGMENTS_SENT.insert($seed, $crate::now!());
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

pub struct FragmentIdentifier {
    set_id: i32,
    total_fragments: u8,
    fragment_id: u8,
}

impl FragmentIdentifier {
    pub fn new(set_id: i32, total_fragments: u8, fragment_id: u8) -> Self {
        Self {
            set_id,
            total_fragments,
            fragment_id,
        }
    }

    pub fn seed(&self) -> i32 {
        let mut seed = self.set_id;
        seed = seed.wrapping_mul(self.total_fragments as i32);
        seed = seed.wrapping_mul(self.fragment_id as i32);
        seed
    }
}

pub struct FragmentTrace {
    fragment_identifier: FragmentIdentifier,
    client_nonce: i32,
}

impl FragmentTrace {
    pub fn new(fragment_identifier: FragmentIdentifier, client_nonce: i32) -> Self {
        Self {
            fragment_identifier,
            client_nonce,
        }
    }

    pub fn seed(&self) -> i32 {
        self.fragment_identifier
            .seed()
            .wrapping_mul(self.client_nonce)
    }
}

lazy_static::lazy_static! {
    pub static ref REGISTRY: MetricsController = MetricsController::default();
    pub static ref FRAGMENTS_RECEIVED: DashMap<u64, FragmentIdentifier> = DashMap::new();
    pub static ref FRAGMENTS_SENT: DashMap<u64, FragmentTrace> = DashMap::new();
}

#[derive(Default)]
pub struct MetricsController {
    registry: Registry,
    registry_index: DashMap<String, Metric>,
}

enum Metric {
    C(Box<IntCounter>),
    G(Box<IntGauge>),
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
    fn inc_by(&self, value: i64) {
        match self {
            Metric::C(c) => c.inc_by(value as u64),
            Metric::G(g) => g.add(value),
        }
    }

    #[inline(always)]
    fn set(&self, value: i64) {
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
            self.register_gauge(Box::new(gauge));
            self.set(name, value)
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
            self.register_counter(Box::new(counter));
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
            self.register_counter(Box::new(counter));
            self.inc_by(name, value)
        }
    }

    fn register_gauge(&self, metric: Box<IntGauge>) {
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

    fn register_counter(&self, metric: Box<IntCounter>) {
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
