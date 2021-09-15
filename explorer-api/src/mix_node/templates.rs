use handlebars::{Handlebars, RenderError};
use serde::Serialize;

#[derive(Clone)]
pub(crate) struct Templates {
    handlebars: Handlebars<'static>,
}

impl Templates {
    pub(crate) fn new() -> Self {
        let mut handlebars = Handlebars::new();

        assert!(handlebars
            .register_template_string("preview", PREVIEW_TEMPLATE)
            .is_ok());

        Templates { handlebars }
    }

    pub(crate) fn render_preview(&self, data: PreviewTemplateData) -> Result<String, RenderError> {
        self.handlebars.render("preview", &data)
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct PreviewTemplateData {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) url: String,
    pub(crate) image_url: String,
}

const PREVIEW_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />

    <title>{{ title }}</title>
    <meta name="description" content="{{ description }}">

    <meta property="og:type" content="article">
    <meta property="og:url" content="{{ url }}">
    <meta property="og:title" content="{{ title }}">
    <meta property="og:description" content="{{ description }}">
    <meta property="og:image" content="{{ image_url }}">

    <meta name="twitter:card" value="summary_large_image">
    <meta name="twitter:title" value="{{ title }}">
    <meta name="twitter:description" value="{{ description }}">
    <meta name="twitter:image" value="{{ image_url }}">
    <meta name="twitter:site" value="@nymtech">

    <meta http-equiv="refresh" content="0;url={{url}}" />
    
    <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol";
    }
    .meme {
      padding-top: 20px;
    }
    .meme > img {
      max-width: 200px;
      max-height: 200px;
    }
    </style>
  </head>
  <body>
    <h1>{{title}}</h1>
    <div>{{description}}<div>
  </body>
</html>"#;
