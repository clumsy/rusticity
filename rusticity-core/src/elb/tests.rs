use super::*;
use aws_sdk_elasticloadbalancingv2::types::{
    FixedResponseActionConfig, ForwardActionConfig, RedirectActionConfig, TargetGroupTuple,
    TargetHealth, TargetHealthStateEnum,
};
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const LB: &str =
    "arn:aws:elasticloadbalancing:us-east-2:123456789012:loadbalancer/app/web/50dc6c495c0c9188";
const TG: &str =
    "arn:aws:elasticloadbalancing:us-east-2:123456789012:targetgroup/web-tg/73e2d6bc24d8a067";
const LISTENER: &str = "arn:aws:elasticloadbalancing:us-east-2:123456789012:listener/app/web/50dc6c495c0c9188/f2f7dc8efc522ab2";
const NS: &str = "http://elasticloadbalancing.amazonaws.com/doc/2015-12-01/";
const DENIED: &str = r#"<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code><Message>no</Message></Error><RequestId>r</RequestId></ErrorResponse>"#;

fn response(action: &str, result: &str) -> String {
    format!(
        r#"<{action}Response xmlns="{NS}"><{action}Result>{result}</{action}Result><ResponseMetadata><RequestId>r</RequestId></ResponseMetadata></{action}Response>"#
    )
}

fn lb_xml(arn: &str) -> String {
    response(
        "DescribeLoadBalancers",
        &format!(
            r#"<LoadBalancers><member>
              <LoadBalancerArn>{arn}</LoadBalancerArn><LoadBalancerName>web</LoadBalancerName>
              <DNSName>web-1.us-east-2.elb.amazonaws.com</DNSName><Type>application</Type>
              <Scheme>internet-facing</Scheme><VpcId>vpc-1</VpcId>
              <State><Code>active</Code></State>
              <SecurityGroups><member>sg-1</member></SecurityGroups>
              <AvailabilityZones><member><ZoneName>us-east-2a</ZoneName><SubnetId>subnet-a</SubnetId></member></AvailabilityZones>
            </member></LoadBalancers>"#
        ),
    )
}

fn tg_xml(arn: &str) -> String {
    response(
        "DescribeTargetGroups",
        &format!(
            r#"<TargetGroups><member>
              <TargetGroupArn>{arn}</TargetGroupArn><TargetGroupName>web-tg</TargetGroupName>
              <Protocol>HTTP</Protocol><Port>80</Port><TargetType>instance</TargetType>
              <HealthCheckPath>/health</HealthCheckPath>
              <Matcher><HttpCode>200</HttpCode></Matcher>
              <LoadBalancerArns><member>{LB}</member></LoadBalancerArns>
            </member></TargetGroups>"#
        ),
    )
}

fn listeners_xml() -> String {
    response(
        "DescribeListeners",
        &format!(
            r#"<Listeners><member>
              <ListenerArn>{LISTENER}</ListenerArn><LoadBalancerArn>{LB}</LoadBalancerArn>
              <Protocol>HTTP</Protocol><Port>80</Port>
              <DefaultActions><member><Type>forward</Type><TargetGroupArn>{TG}</TargetGroupArn></member></DefaultActions>
            </member></Listeners>"#
        ),
    )
}

fn rules_xml() -> String {
    response(
        "DescribeRules",
        &format!(
            r#"<Rules>
              <member><RuleArn>r1</RuleArn><Priority>default</Priority><IsDefault>true</IsDefault>
                <Actions><member><Type>forward</Type><TargetGroupArn>{TG}</TargetGroupArn></member></Actions></member>
              <member><RuleArn>r2</RuleArn><Priority>10</Priority><IsDefault>false</IsDefault>
                <Actions><member><Type>fixed-response</Type><FixedResponseConfig><StatusCode>404</StatusCode></FixedResponseConfig></member></Actions></member>
            </Rules>"#
        ),
    )
}

fn health_xml() -> String {
    response(
        "DescribeTargetHealth",
        r#"<TargetHealthDescriptions>
          <member><Target><Id>i-0123456789abcdef0</Id><Port>80</Port></Target><TargetHealth><State>healthy</State></TargetHealth></member>
          <member><Target><Id>i-0fffffffffffffff0</Id><Port>80</Port></Target><TargetHealth><State>unhealthy</State><Reason>Target.FailedHealthChecks</Reason><Description>Health checks failed</Description></TargetHealth></member>
        </TargetHealthDescriptions>"#,
    )
}

/// Answers each request from `route(action)` and records each request body.
fn routed(
    route: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> (AwsConfig, Arc<Mutex<Vec<String>>>) {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let record = seen.clone();
    let http = infallible_client_fn(move |request| {
        let sent = request
            .body()
            .bytes()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let action = sent
            .split('&')
            .find_map(|pair| pair.strip_prefix("Action="))
            .unwrap_or_default()
            .to_string();
        record.lock().unwrap().push(sent);
        let (status, body) = route(&action);
        http::Response::builder()
            .status(status)
            .body(body.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-2"))
        .credentials_provider(
            aws_sdk_elasticloadbalancingv2::config::SharedCredentialsProvider::new(
                aws_sdk_elasticloadbalancingv2::config::Credentials::new(
                    "test", "test", None, None, "test",
                ),
            ),
        )
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    (
        AwsConfig::from_sdk_config(sdk_config, "us-east-2", "123456789012"),
        seen,
    )
}

fn count(seen: &Arc<Mutex<Vec<String>>>, action: &str) -> usize {
    seen.lock()
        .unwrap()
        .iter()
        .filter(|r| r.contains(&format!("Action={action}&")))
        .count()
}

#[tokio::test]
async fn load_balancer_detail_reads_one_by_arn_and_builds_listeners_and_routes() {
    let (config, seen) = routed(|action| match action {
        "DescribeLoadBalancers" => (200, lb_xml(LB)),
        "DescribeListeners" => (200, listeners_xml()),
        "DescribeTargetGroups" => (200, tg_xml(TG)),
        "DescribeRules" => (200, rules_xml()),
        "DescribeTargetHealth" => (200, health_xml()),
        _ => (403, DENIED.into()),
    });
    let d = ElbClient::new(config)
        .load_balancer_details(LB)
        .await
        .unwrap();
    assert_eq!(d.load_balancer.name, "web");
    assert_eq!(d.load_balancer.zones[0].subnet_id, "subnet-a");
    let listeners = d.listeners.unwrap();
    assert_eq!(listeners.len(), 1);
    assert_eq!(
        listeners[0].default_action,
        "Forward to target group: web-tg (100%)"
    );
    assert_eq!(listeners[0].rules, Some(2));
    let routes = d.resource_map.unwrap();
    assert_eq!(routes.len(), 1, "the fixed response rule forwards nowhere");
    let r = &routes[0];
    assert_eq!(
        (r.listener.as_str(), r.rule.as_str()),
        ("HTTP:80", "Default")
    );
    assert_eq!(r.target_group, "web-tg");
    assert_eq!(
        (r.targets, r.healthy, r.unhealthy),
        (Some(2), Some(1), Some(1))
    );
    // Denied sections fail alone.
    assert!(d
        .security_groups
        .unwrap_err()
        .starts_with("Security groups unavailable"));
    assert!(d
        .attributes
        .unwrap_err()
        .starts_with("Attributes unavailable"));
    assert!(d.tags.is_err() && d.monitoring.is_err());
    assert_eq!(
        count(&seen, "DescribeLoadBalancers"),
        1,
        "exactly one identity read"
    );
    let seen = seen.lock().unwrap();
    let first = &seen[0];
    assert!(first.contains("Action=DescribeLoadBalancers&"), "{first}");
    assert!(first.contains("LoadBalancerArns.member.1="), "{first}");
}

#[tokio::test]
async fn a_failed_listener_read_fails_listeners_and_the_resource_map_only() {
    let (config, _) = routed(|action| match action {
        "DescribeLoadBalancers" => (200, lb_xml(LB)),
        "DescribeTags" => (
            200,
            response(
                "DescribeTags",
                &format!(
                    r#"<TagDescriptions><member><ResourceArn>{LB}</ResourceArn><Tags><member><Key>team</Key><Value>web</Value></member></Tags></member></TagDescriptions>"#
                ),
            ),
        ),
        _ => (403, DENIED.into()),
    });
    let d = ElbClient::new(config)
        .load_balancer_details(LB)
        .await
        .unwrap();
    assert!(d
        .listeners
        .unwrap_err()
        .starts_with("Listeners unavailable"));
    assert!(d.resource_map.is_err());
    assert_eq!(d.tags.unwrap(), [("team".to_string(), "web".to_string())]);
}

#[tokio::test]
async fn mismatched_identity_is_rejected() {
    let other = LB.replace("/web/", "/other/");
    let (config, _) = routed(move |_| (200, lb_xml(&other)));
    let error = ElbClient::new(config)
        .load_balancer_details(LB)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("mismatched identity"), "{error}");
}

#[tokio::test]
async fn foreign_or_malformed_arns_are_rejected_before_any_request() {
    let (config, seen) = routed(|_| (200, lb_xml(LB)));
    let client = ElbClient::new(config);
    for arn in [
        LB.replace("us-east-2", "eu-west-1"),
        LB.replace("123456789012", "999999999999"),
        TG.to_string(),
        "web".to_string(),
        format!("{LB}\n"),
    ] {
        assert!(client.load_balancer_details(&arn).await.is_err(), "{arn}");
    }
    assert!(client.target_group_details(LB).await.is_err());
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn target_group_detail_reads_targets_with_health_and_instance_names() {
    let (config, seen) = routed(|action| {
        match action {
        "DescribeTargetGroups" => (200, tg_xml(TG)),
        "DescribeTargetHealth" => (200, health_xml()),
        "DescribeInstances" => (
            200,
            r#"<DescribeInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/"><reservationSet><item><instancesSet><item><instanceId>i-0123456789abcdef0</instanceId><tagSet><item><key>Name</key><value>web-1</value></item></tagSet></item></instancesSet></item></reservationSet></DescribeInstancesResponse>"#.into(),
        ),
        _ => (403, DENIED.into()),
    }
    });
    let d = ElbClient::new(config)
        .target_group_details(TG)
        .await
        .unwrap();
    assert_eq!(d.target_group.health_check.path, "/health");
    assert_eq!(d.target_group.health_check.success_codes, "200");
    let targets = d.targets.unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(
        (targets[0].name.as_str(), targets[0].state.as_str()),
        ("web-1", "healthy")
    );
    assert_eq!(targets[1].name, "", "no name tag, nothing invented");
    assert_eq!(targets[1].reason, "Target.FailedHealthChecks");
    assert!(d.attributes.is_err() && d.tags.is_err() && d.monitoring.is_err());
    assert_eq!(count(&seen, "DescribeTargetGroups"), 1);
}

#[tokio::test]
async fn denied_target_health_fails_only_the_targets() {
    let (config, _) = routed(|action| match action {
        "DescribeTargetGroups" => (200, tg_xml(TG)),
        _ => (403, DENIED.into()),
    });
    let d = ElbClient::new(config)
        .target_group_details(TG)
        .await
        .unwrap();
    assert!(d.targets.unwrap_err().starts_with("Targets unavailable"));
    assert_eq!(d.target_group.name, "web-tg");
}

#[test]
fn arns_give_names_and_metric_dimensions() {
    assert_eq!(name_from_arn(LB), "web");
    assert_eq!(name_from_arn(TG), "web-tg");
    assert_eq!(name_from_arn("web"), "");
    assert_eq!(dimension(LB), "app/web/50dc6c495c0c9188");
    assert_eq!(dimension(TG), "targetgroup/web-tg/73e2d6bc24d8a067");
}

#[test]
fn actions_read_as_in_the_console() {
    let names = |arn: &str| name_from_arn(arn);
    let weighted = Action::builder()
        .r#type(ActionTypeEnum::Forward)
        .forward_config(
            ForwardActionConfig::builder()
                .target_groups(
                    TargetGroupTuple::builder()
                        .target_group_arn(TG)
                        .weight(3)
                        .build(),
                )
                .target_groups(
                    TargetGroupTuple::builder()
                        .target_group_arn(TG.replace("web-tg", "canary"))
                        .weight(1)
                        .build(),
                )
                .build(),
        )
        .build();
    assert_eq!(
        action(&weighted, &names),
        "Forward to target group: web-tg (75%), canary (25%)"
    );
    let redirect = Action::builder()
        .r#type(ActionTypeEnum::Redirect)
        .redirect_config(
            RedirectActionConfig::builder()
                .protocol("HTTPS")
                .port("443")
                .status_code(
                    aws_sdk_elasticloadbalancingv2::types::RedirectActionStatusCodeEnum::Http301,
                )
                .build(),
        )
        .build();
    assert_eq!(
        action(&redirect, &names),
        "Redirect to HTTPS://#{host}:443/#{path}?#{query}"
    );
    let fixed = Action::builder()
        .r#type(ActionTypeEnum::FixedResponse)
        .fixed_response_config(
            FixedResponseActionConfig::builder()
                .status_code("503")
                .build(),
        )
        .build();
    assert_eq!(action(&fixed, &names), "Return fixed response: 503");
}

#[test]
fn health_counts_split_healthy_and_unhealthy() {
    let state = |s| {
        TargetHealthDescription::builder()
            .target_health(TargetHealth::builder().state(s).build())
            .build()
    };
    let targets = [
        state(TargetHealthStateEnum::Healthy),
        state(TargetHealthStateEnum::Unhealthy),
        state(TargetHealthStateEnum::UnhealthyDraining),
        state(TargetHealthStateEnum::Initial),
    ];
    assert_eq!(health_counts(&targets), (4, 1, 2));
}
