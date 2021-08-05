use rocket_okapi::swagger_ui::{SwaggerUIConfig, UrlObject};

pub(crate) fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        urls: vec![
            UrlObject::new("Country statistics", "/countries/openapi.json"),
            UrlObject::new("Node ping", "/ping/openapi.json"),
        ],
        ..Default::default()
    }
}
