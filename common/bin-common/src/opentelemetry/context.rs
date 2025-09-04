use opentelemetry::propagation::{Injector, Extractor};
use std::collections::HashMap;

/// Make a Carrier for context propagation
pub struct ContextCarrier {
    data: HashMap<String, String>,
}

impl ContextCarrier {
    pub fn new() -> Self {
        ContextCarrier {
            data: HashMap::new(),
        }
    }

    pub fn from_map(data: HashMap<String, String>) -> Self {
        ContextCarrier { data }
    }

    pub fn into_map(self) -> HashMap<String, String> {
        self.data
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

// TODO use generics to extract context from request
    // fn extract_context_from_request<T>(request: ) -> Context {
    //     global::get_text_map_propagator(|propagator| {
    //         propagator.extract(&HeaderExtractor(&request.headers))
    //     })
    // }

