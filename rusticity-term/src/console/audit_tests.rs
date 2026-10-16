//! Additional regressions discovered by the cross-service console audit.
use crate::apig::{api::RestApi, resource::Resource, route::Route};
use crate::app::{App, Service};
use crate::ui::apig::ApiDetailTab;

fn api(protocol: &str) -> RestApi {
    RestApi {
        id: "api123".into(),
        name: "api".into(),
        description: String::new(),
        created_date: String::new(),
        api_key_source: String::new(),
        endpoint_configuration: String::new(),
        protocol_type: protocol.into(),
        disable_execute_api_endpoint: false,
        status: String::new(),
    }
}
fn app(protocol: &str) -> App {
    let mut app = App::new_without_client("test".into(), Some("eu-west-1".into()));
    app.current_service = Service::ApiGatewayApis;
    app.apig_state.current_api = Some(api(protocol));
    app.apig_state.detail_tab = ApiDetailTab::Routes;
    app
}
fn resource(id: &str, label: &str, expandable: bool) -> Resource {
    Resource {
        id: id.into(),
        path: label.into(),
        display_name: label.into(),
        parent_id: None,
        methods: if expandable {
            vec!["GET".into()]
        } else {
            vec![]
        },
        arn: String::new(),
    }
}

#[test]
fn rest_resources_link_expanded_children_and_filtered_visible_rows() {
    let mut app = app("REST");
    app.apig_state.resources.items = vec![resource("root", "/", true)];
    app.apig_state.resource_children.insert(
        "root".into(),
        vec![
            resource("hidden", "DELETE", false),
            resource("visible", "GET", true),
        ],
    );
    app.apig_state.resource_children.insert(
        "visible".into(),
        vec![resource("leaf+id", "GET leaf", false)],
    );
    app.apig_state
        .expanded_resources
        .extend(["root".into(), "visible".into()]);
    let base = "https://eu-west-1.console.aws.amazon.com/apigateway/main/apis/api123/resources?api=api123&region=eu-west-1";
    for (selected, id) in [(0, "root"), (1, "hidden"), (2, "visible"), (3, "leaf%2Bid")] {
        app.apig_state.resources.selected = selected;
        assert_eq!(app.get_console_url(), format!("{base}#{id}"));
    }
    app.apig_state.route_filter = "GET".into();
    // The raw first child is hidden; visible index 1 must select 'visible'.
    for (selected, id) in [(0, "root"), (1, "visible"), (2, "leaf%2Bid")] {
        app.apig_state.resources.selected = selected;
        assert_eq!(app.get_console_url(), format!("{base}#{id}"));
    }
    app.apig_state.resources.selected = 999;
    assert_eq!(app.get_console_url(), base);
    app.apig_state.resources.selected = 1;
    app.apig_state.expanded_resources.clear();
    assert_eq!(app.get_console_url(), base);
    app.apig_state.route_filter = "missing".into();
    app.apig_state.resources.selected = 0;
    assert_eq!(app.get_console_url(), base);
}

#[test]
fn http_and_websocket_routes_use_filtered_children_not_raw_positions() {
    fn route(id: &str, key: &str, target: &str) -> Route {
        Route {
            route_id: id.into(),
            route_key: key.into(),
            display_name: key.into(),
            target: target.into(),
            authorization_type: String::new(),
            api_key_required: false,
            arn: String::new(),
        }
    }
    for protocol in ["HTTP", "WEBSOCKET"] {
        let mut app = app(protocol);
        app.apig_state.routes.items = vec![route("", "/", "")];
        app.apig_state.route_children.insert(
            "/".into(),
            vec![
                route("hidden", "DELETE /", "integration/a"),
                route("visible", "GET /", "integration/b"),
            ],
        );
        app.apig_state.expanded_routes.insert("/".into());
        let base = "https://eu-west-1.console.aws.amazon.com/apigateway/main/develop/routes?api=api123&region=eu-west-1";
        app.apig_state.routes.selected = 2;
        assert_eq!(app.get_console_url(), format!("{base}&routes=visible"));
        app.apig_state.route_filter = "GET".into();
        app.apig_state.routes.selected = 1;
        assert_eq!(app.get_console_url(), format!("{base}&routes=visible"));
        app.apig_state.routes.selected = 0;
        assert_eq!(app.get_console_url(), base); // synthetic parent has no target
        app.apig_state.routes.selected = 999;
        assert_eq!(app.get_console_url(), base);
    }
}

#[test]
fn apig_closed_detail_restores_api_list_despite_highlighted_row() {
    let mut app = app("REST");
    app.apig_state.current_api = None;
    app.apig_state.apis.items = vec![api("REST")];
    assert_eq!(
        app.get_console_url(),
        "https://eu-west-1.console.aws.amazon.com/apigateway/main/apis?region=eu-west-1"
    );
}

#[test]
fn lambda_application_details_and_orphan_version_routing() {
    use crate::ui::lambda::ApplicationDetailTab;
    let mut app = App::new_without_client("test".into(), Some("eu-west-1".into()));
    app.current_service = Service::LambdaApplications;
    app.lambda_application_state.current_application = Some("app/a+b".into());
    for (tab, label) in [
        (ApplicationDetailTab::Overview, "overview"),
        (ApplicationDetailTab::Deployments, "deployments"),
    ] {
        app.lambda_application_state.detail_tab = tab;
        assert_eq!(app.get_console_url(), format!("https://eu-west-1.console.aws.amazon.com/lambda/home?region=eu-west-1#/applications/app%2Fa%2Bb?tab={label}"));
    }
    app.current_service = Service::LambdaFunctions;
    app.lambda_state.current_version = Some("4".into());
    app.lambda_state.current_function = None;
    assert_eq!(
        app.get_console_url(),
        "https://eu-west-1.console.aws.amazon.com/lambda/home?region=eu-west-1#/functions"
    );
}

#[test]
fn alarm_detail_overrides_filtered_list_and_return_restores_list_route() {
    let mut app = App::new_without_client("test".into(), Some("eu-west-1".into()));
    app.current_service = Service::CloudWatchAlarms;
    app.alarms_state.table.filter = "no rows".into();
    app.alarms_state.table.selected = usize::MAX;
    app.alarms_state.current_alarm = Some("alarm/+%name".into());
    assert!(app
        .get_console_url()
        .contains("#alarmsV2:alarm/alarm%2F%2B%25name?"));
    app.alarms_state.current_alarm = None;
    assert!(app.get_console_url().contains("#alarmsV2:alarms?"));
}

#[test]
fn insights_sources_use_the_active_partition_not_hardcoded_aws() {
    for (region, partition, host) in [
        ("us-east-1", "aws", "us-east-1.console.aws.amazon.com"),
        ("cn-north-1", "aws-cn", "cn-north-1.console.amazonaws.cn"),
        (
            "us-gov-west-1",
            "aws-us-gov",
            "console.amazonaws-us-gov.com",
        ),
    ] {
        let url =
            crate::cw::insights::console_url(region, "123456789012", "", &["/aws/demo".into()]);
        assert!(url.starts_with(&format!("https://{host}/cloudwatch/home?region={region}#")));
        assert!(url.contains(&format!(
            "arn*3a{partition}*3alogs*3a{region}*3a123456789012*3alog-group*3a*2faws*2fdemo"
        )));
    }
}
