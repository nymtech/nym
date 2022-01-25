use rocket_okapi::swagger_ui::SwaggerUIConfig;

pub(crate) fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "../v1/openapi.json".to_owned(),
        ..Default::default()
    }
}
