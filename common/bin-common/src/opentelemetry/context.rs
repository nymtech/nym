use opentelemetry::{Context, TraceFlags};
use opentelemetry::propagation::{Injector, Extractor, TextMapPropagator};
use opentelemetry::trace::{SpanContext, TraceContextExt, TraceId};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::IdGenerator};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use std::collections::HashMap;
use std::fmt::Display;
use tracing::instrument;

/// Make a Carrier for context propagation
pub struct ContextCarrier {
    data: HashMap<String, String>,
}

impl ContextCarrier {
    pub fn new_empty() -> Self {
        ContextCarrier {
            data: HashMap::new(),
        }
    }

    pub fn new_with_data(data: HashMap<String, String>) -> Self {
        if data.is_empty() {
            return ContextCarrier::new_empty();
        }
        
        ContextCarrier { data }
    }

    pub fn new_with_current_context(context: Context) -> Self {
        let mut carrier = ContextCarrier::new_empty();
        let propagator = TraceContextPropagator::new();
        propagator.inject_context(&context, &mut carrier);
        carrier
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.data.iter()
    }

    pub fn from_map(data: HashMap<String, String>) -> Self {
        ContextCarrier { data }
    }

    pub fn into_map(self) -> HashMap<String, String> {
        self.data
    }

    pub fn extract_trace_id(&self) -> Option<TraceId> {
        self.get("traceparent").and_then(|tp| {
            let parts: Vec<&str> = tp.split('-').collect();
            if parts.len() == 4 {
                TraceId::from_hex(parts[1]).ok()
            } else {
                None
            }
        })
    }

    pub fn extract_trace_id_into_bytes(&self) -> Option<[u8; 16]> {
        self.extract_trace_id().map(|id| id.to_bytes())
    }

    pub fn extract_traceparent(&self) -> Option<String> {
        self.get("traceparent").map(|s| s.to_string())
    }
}

impl Injector for ContextCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.data.insert(key.to_string(), value);
    }
}

impl Extractor for ContextCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.data.keys().map(|k| k.as_str()).collect()
    }
}

impl Display for ContextCarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.data)
    }
}

pub struct ManualContextPropagator {
    pub root_span: tracing::Span,
    pub trace_id: TraceId,
}

impl ManualContextPropagator {
    #[instrument(skip_all, level = "debug")]
    pub fn new(name: &str, context: HashMap<String, String>) -> Self {
        let carrier = ContextCarrier::new_with_data(context);
        let trace_id = match carrier.extract_trace_id() {
            Some(id) => id,
            None => Context::current().span().span_context().trace_id(),
        };

        let root_span_builder = new_span_context_with_id(trace_id.clone());

        let root_span = tracing::info_span!("trace_root", name = %name, trace_id = %trace_id);
        root_span.set_parent(root_span_builder);

        ManualContextPropagator {
            root_span,
            trace_id,
        }
    }

    #[instrument(skip_all, level = "debug")]
    pub fn new_from_tid(name: &str, trace_id: TraceId) -> Self {
        let root_span_builder = new_span_context_with_id(trace_id.clone());

        let root_span = tracing::info_span!("trace_root", name = %name, trace_id = %trace_id);
        root_span.set_parent(root_span_builder);

        ManualContextPropagator {
            root_span,
            trace_id,
        }
    }

    pub fn root_span(&self) -> &tracing::Span {
        &self.root_span
    }

}

#[instrument(skip_all, level = "debug")]
pub fn new_span_context_with_id(trace_id: TraceId) -> Context {
    let id_gen = opentelemetry_sdk::trace::RandomIdGenerator::default();
    let span_id = id_gen.new_span_id();
    let span_context = SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::SAMPLED,
        true,
        Default::default(),
    );

    Context::current().with_remote_span_context(span_context)
}

#[instrument(skip_all, level = "debug")]
pub fn extract_trace_id_from_tracing_cx() -> TraceId {
    let cx = tracing::Span::current().context();
    let binding = cx.span();
    let trace_id = binding.span_context().trace_id();
    trace_id
}