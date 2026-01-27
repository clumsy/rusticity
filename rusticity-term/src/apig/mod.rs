pub mod api;
pub mod resource;
pub mod route;
use ratatui::style::Color;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    api::init(i18n);
}

pub fn format_status(status: &str) -> (String, Color) {
    match status.to_uppercase().as_str() {
        "AVAILABLE" => ("✅ Available".to_string(), Color::Green),
        _ => (status.to_string(), Color::White),
    }
}

pub fn console_url_apis(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/apigateway/main/apis?region={}",
        region, region
    )
}

pub fn console_url_api(region: &str, api_id: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/apigateway/main/api-detail?api={}&region={}",
        region, api_id, region
    )
}

pub fn console_url_resources(region: &str, api_id: &str, resource_id: Option<&str>) -> String {
    let mut url = format!(
        "https://{}.console.aws.amazon.com/apigateway/main/apis/{}/resources?api={}&region={}",
        region, api_id, api_id, region
    );
    if let Some(rid) = resource_id {
        url.push_str(&format!("#{}", rid));
    }
    url
}

pub fn console_url_routes(region: &str, api_id: &str, route_id: Option<&str>) -> String {
    let mut url = format!(
        "https://{}.console.aws.amazon.com/apigateway/main/develop/routes?api={}&region={}",
        region, api_id, region
    );
    if let Some(rid) = route_id {
        url.push_str(&format!("&routes={}", rid));
    }
    url
}
