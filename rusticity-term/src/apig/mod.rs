pub mod actions;
pub mod api;
pub mod resource;
pub mod route;
use crate::console::{component, ConsoleUrl};
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
    ConsoleUrl::regional(region, "/apigateway/main/apis")
        .query("region", region)
        .finish()
}

pub fn console_url_api(region: &str, api_id: &str) -> String {
    ConsoleUrl::regional(region, "/apigateway/main/api-detail")
        .query("api", api_id)
        .query("region", region)
        .finish()
}

pub fn console_url_resources(region: &str, api_id: &str, resource_id: Option<&str>) -> String {
    let mut url = ConsoleUrl::regional(
        region,
        &format!("/apigateway/main/apis/{}/resources", component(api_id)),
    )
    .query("api", api_id)
    .query("region", region);
    if let Some(rid) = resource_id {
        url = url.fragment(component(rid).into_owned());
    }
    url.finish()
}

pub fn console_url_routes(region: &str, api_id: &str, route_id: Option<&str>) -> String {
    let mut url = ConsoleUrl::regional(region, "/apigateway/main/develop/routes")
        .query("api", api_id)
        .query("region", region);
    if let Some(rid) = route_id {
        url = url.query("routes", rid);
    }
    url.finish()
}

#[cfg(test)]
mod console_url_tests {
    use super::*;

    #[test]
    fn api_query_order_preserved_api_then_region() {
        assert_eq!(
            console_url_api("us-east-1", "abc123"),
            "https://us-east-1.console.aws.amazon.com/apigateway/main/api-detail?api=abc123&region=us-east-1"
        );
    }

    #[test]
    fn resources_encodes_api_id_in_path_and_resource_id_in_fragment() {
        let url = console_url_resources("eu-west-1", "api/x", Some("res id+1"));
        assert_eq!(
            url,
            "https://eu-west-1.console.aws.amazon.com/apigateway/main/apis/api%2Fx/resources?api=api%2Fx&region=eu-west-1#res%20id%2B1"
        );
    }

    #[test]
    fn routes_adds_route_id_as_raw_query_value() {
        let url = console_url_routes("us-west-2", "a b", Some("GET /x"));
        assert_eq!(
            url,
            "https://us-west-2.console.aws.amazon.com/apigateway/main/develop/routes?api=a%20b&region=us-west-2&routes=GET%20%2Fx"
        );
    }
}
