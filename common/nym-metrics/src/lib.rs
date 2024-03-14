use log::debug;
use std::fmt;

use prometheus::{core::Collector, Encoder as _, Registry, TextEncoder};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[derive(Default)]
pub struct MetricsController {
    registry: Registry,
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
    pub fn register(&self, metric: Box<dyn Collector>) {
        let fq_name = metric
            .desc()
            .first()
            .map(|d| d.fq_name.clone())
            .unwrap_or_default();
        match self.registry.register(metric) {
            Ok(_) => {}
            Err(e) => {
                debug!("Failed to register {:?}:\n{}", fq_name, e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
