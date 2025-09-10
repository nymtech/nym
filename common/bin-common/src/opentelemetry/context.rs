use opentelemetry::{Context, TraceFlags};
use opentelemetry::propagation::{Injector, Extractor, TextMapPropagator};
use opentelemetry::trace::{SpanContext, TraceContextExt, TraceId};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::IdGenerator;
use std::collections::HashMap;

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

    pub fn new_with_current_context(context: Context) -> Self {
        let propagator = TraceContextPropagator::new();
        let mut carrier = ContextCarrier::new_empty();
        propagator.inject_context(&context, &mut carrier);
        carrier
    }

    pub fn new_with_extracted_context(external_context: &ContextCarrier) -> Self {
        let extractor = TraceContextPropagator::new();
        let context = extractor.extract(external_context);
        ContextCarrier::new_with_current_context(context)
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

pub struct AsyncSpanContextExt {
    pub context_carrier: ContextCarrier,
    pub root_span: tracing::Span,
    pub trace_id: Option<TraceId>,
}

impl AsyncSpanContextExt {
    pub fn new() -> Self {
        AsyncSpanContextExt {
            context_carrier: ContextCarrier::new_empty(),
            root_span: tracing::Span::none(),
            trace_id: None,
        }
    }

    pub fn with_context_carrier(mut self, carrier: ContextCarrier) -> Self {
        self.context_carrier = carrier;
        self.trace_id = self.context_carrier.extract_trace_id();
        self
    }

    pub fn with_extracted_context(mut self, external_context: &ContextCarrier) -> Self {
        self.context_carrier = ContextCarrier::new_with_extracted_context(external_context);
        self.trace_id = self.context_carrier.extract_trace_id();
        self
    }

    pub fn set_root_span(mut self, span: tracing::Span) -> Self {
        self.root_span = span;
        self
    }
 }

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