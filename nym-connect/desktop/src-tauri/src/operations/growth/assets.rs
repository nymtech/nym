use rust_embed::RustEmbed;
extern crate yaml_rust;
use yaml_rust::YamlLoader;

#[derive(RustEmbed)]
#[folder = "../src/components/Growth/content/"]
#[include = "*.yaml"]
#[exclude = "*.mdx"]
struct Asset;

#[derive(Debug)]
pub struct NotificationContent {
    pub title: String,
    pub body: String,
}

#[derive(Debug)]
pub struct Notifications {
    #[allow(dead_code)]
    pub you_are_in_draw: NotificationContent,
    pub take_part: NotificationContent,
}

pub struct Content {}

const RESOURCE_ERROR: &str = "❌ RESOURCE ERROR";

fn get_as_string_or_error_message(value: &yaml_rust::Yaml) -> String {
    value.as_str().unwrap_or(RESOURCE_ERROR).to_string()
}

impl Content {
    pub fn get_notifications() -> Notifications {
        let content = Asset::get("en.yaml").unwrap();
        let s = std::str::from_utf8(content.data.as_ref()).unwrap();
        let content = YamlLoader::load_from_str(s).unwrap();
        let content = &content[0];

        Notifications {
            you_are_in_draw: NotificationContent {
                title: get_as_string_or_error_message(
                    &content["testAndEarn"]["notifications"]["youAreInDraw"]["title"],
                ),
                body: get_as_string_or_error_message(
                    &content["testAndEarn"]["notifications"]["youAreInDraw"]["body"],
                ),
            },
            take_part: NotificationContent {
                title: get_as_string_or_error_message(
                    &content["testAndEarn"]["notifications"]["takePart"]["title"],
                ),
                body: get_as_string_or_error_message(
                    &content["testAndEarn"]["notifications"]["takePart"]["body"],
                ),
            },
        }
    }
}
