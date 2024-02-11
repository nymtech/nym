use std::collections::HashMap;

use prometheus::{Gauge, Registry};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::MetricsError;

// Generic helper to recursively register prometheus metrics from a given struct, nested fields are handled by by concating field names with a dot
// Integers and floats are registered as [gauges](https://prometheus.io/docs/concepts/metric_types/#gauge),
// since gauge api is more robust and can be used to display counters as well
pub struct PrometheusMetrics<'a, T: Serialize + 'a> {
    registry: Registry,
    source: &'a T,
    metrics: HashMap<String, Gauge>,
}

struct MetricValue {
    name: String,
    value: f64,
}

struct PromMetric {
    name: String,
    gauge: Gauge,
}

impl<'a, T: Serialize> PrometheusMetrics<'a, T> {
    pub fn init(registry: Registry, source: &'a T) -> Result<Self, MetricsError> {
        let mut p = PrometheusMetrics {
            registry,
            source,
            metrics: HashMap::new(),
        };
        let metrics = p.init_metrics()?;
        for metric in metrics {
            p.registry.register(Box::new(metric.gauge.clone()))?;
            p.metrics.insert(metric.name, metric.gauge);
        }
        Ok(p)
    }

    pub fn update(&self) -> Result<(), MetricsError> {
        let metrics_map: Map<String, Value> = serde_json::to_value(self.source)?
            .as_object()
            .ok_or_else(|| MetricsError::NotAnObject)?
            .clone();
        let metric_values = flatten_all(metrics_map.iter().map(|(k, v)| collect_metric(k, v)))?;
        for metric in metric_values {
            self.metrics
                .get(&metric.name)
                .ok_or_else(|| MetricsError::PrometheusError {
                    source: prometheus::Error::Msg(format!(
                        "metric {} not found in the registry",
                        metric.name
                    )),
                })?
                .set(metric.value);
        }
        Ok(())
    }

    pub fn init_metrics(&self) -> Result<Vec<PromMetric>, MetricsError> {
        let metrics_map: Map<String, Value> = serde_json::to_value(self.source)?
            .as_object()
            .ok_or_else(|| MetricsError::NotAnObject)?
            .clone();
        let metrics = flatten_all(metrics_map.iter().map(|(k, v)| init_metric(k, v)))?;
        Ok(metrics)
    }
}

fn init_gauge(name: &str) -> Result<PromMetric, MetricsError> {
    Ok(PromMetric {
        name: name.to_string(),
        gauge: prometheus::Gauge::new(name, "")?,
    })
}

fn init_metric(name: &str, value: &Value) -> Result<Option<Vec<PromMetric>>, MetricsError> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Null => Ok(None),
        Value::Number(v) => Ok(Some(vec![init_gauge(name)?])),
        Value::Array(arr) => {
            Ok(Some(flatten_all(arr.iter().enumerate().map(|(i, v)| {
                init_metric(&format!("{}.{}", name, i), v)
            }))?))
        }
        Value::Object(obj) => {
            Ok(Some(flatten_all(obj.iter().map(|(k, v)| {
                init_metric(&format!("{}.{}", name, k), v)
            }))?))
        }
    }
}

fn collect_metric(name: &str, value: &Value) -> Result<Option<Vec<MetricValue>>, MetricsError> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Null => Ok(None),
        Value::Number(v) => Ok(Some(vec![MetricValue {
            name: name.to_string(),
            value: v.as_f64().ok_or_else(|| MetricsError::NotANumber)?,
        }])),
        Value::Array(arr) => {
            Ok(Some(flatten_all(arr.iter().enumerate().map(|(i, v)| {
                collect_metric(&format!("{}.{}", name, i), v)
            }))?))
        }
        Value::Object(obj) => {
            Ok(Some(flatten_all(obj.iter().map(|(k, v)| {
                collect_metric(&format!("{}.{}", name, k), v)
            }))?))
        }
    }
}

fn flatten_all<T>(
    it: impl Iterator<Item = Result<Option<Vec<T>>, MetricsError>>,
) -> Result<Vec<T>, MetricsError> {
    Ok(it
        .collect::<Result<Vec<Option<Vec<T>>>, MetricsError>>()?
        .into_iter()
        .filter_map(|x| x)
        .flatten()
        .collect::<Vec<T>>())
}
