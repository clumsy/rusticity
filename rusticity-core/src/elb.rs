//! Elastic Load Balancing (v2): load balancers and target groups, each a list
//! and a detail. A detail read describes the resource by its ARN and verifies
//! the returned identity, then runs bounded parallel reads for the tab
//! sections; a failed section fails only itself. Read-only.
use crate::config::AwsConfig;
use crate::metrics::{MetricQuery, MetricReader};
use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_cloudwatch::types::Statistic;
use aws_sdk_elasticloadbalancingv2::types::{
    Action, ActionTypeEnum, Listener as SdkListener, LoadBalancer as SdkLoadBalancer,
    TargetGroup as SdkTargetGroup, TargetHealthDescription,
};
use aws_smithy_types::date_time::Format;
use std::future::Future;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
/// Bound on each section read after the resource read succeeded.
const ENRICH_TIMEOUT: Duration = Duration::from_secs(15);

/// One section of a detail tab: its rows, or why they are unavailable.
pub type Section<T> = Result<Vec<T>, String>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZoneMapping {
    pub zone: String,
    pub subnet_id: String,
    /// Elastic or AWS-assigned IPv4 addresses (Network Load Balancer).
    pub ipv4: Vec<String>,
    pub private_ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}

/// One load balancer; raw API values, named by the view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadBalancer {
    pub name: String,
    pub arn: String,
    pub dns_name: String,
    /// `active`, `provisioning`, `active_impaired` or `failed`.
    pub state: String,
    pub state_reason: String,
    /// `application`, `network` or `gateway`.
    pub lb_type: String,
    /// `internet-facing` or `internal`.
    pub scheme: String,
    pub vpc_id: String,
    pub zones: Vec<ZoneMapping>,
    /// `ipv4`, `dualstack` or `dualstack-without-public-ipv4`.
    pub ip_address_type: String,
    /// ISO 8601; empty when absent.
    pub created_time: String,
    pub security_groups: Vec<String>,
    pub hosted_zone: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HealthCheck {
    pub enabled: Option<bool>,
    pub protocol: String,
    pub port: String,
    pub path: String,
    pub healthy_threshold: Option<i32>,
    pub unhealthy_threshold: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub interval_seconds: Option<i32>,
    /// HTTP or gRPC success codes.
    pub success_codes: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TargetGroup {
    pub name: String,
    pub arn: String,
    pub port: Option<i32>,
    pub protocol: String,
    pub protocol_version: String,
    /// `instance`, `ip`, `lambda` or `alb`.
    pub target_type: String,
    pub vpc_id: String,
    pub ip_address_type: String,
    pub load_balancer_arns: Vec<String>,
    pub health_check: HealthCheck,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listener {
    pub arn: String,
    pub protocol: String,
    pub port: Option<i32>,
    /// `Forward to target group: web (100%)`, `Redirect to HTTPS://...:443`.
    pub default_action: String,
    /// Rules on the listener, the default rule included; `None` for a type
    /// without rules or when the rules read failed.
    pub rules: Option<usize>,
    pub ssl_policy: String,
    pub default_certificate: String,
    /// `off`, `passthrough` or `verify`; empty when not reported.
    pub mtls_mode: String,
    pub trust_store_arn: String,
}

/// One route on the Resource map: listener, rule, target group and the
/// health of its targets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceRoute {
    /// `HTTP:80`.
    pub listener: String,
    /// `Default` or the rule priority.
    pub rule: String,
    pub target_group: String,
    pub target_group_arn: String,
    /// Target health counts; `None` when the target health read failed.
    pub targets: Option<usize>,
    pub healthy: Option<usize>,
    pub unhealthy: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecurityGroup {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Target {
    pub id: String,
    /// The instance `Name` tag; empty for other target types.
    pub name: String,
    pub port: Option<i32>,
    pub zone: String,
    /// Raw `TargetHealthStateEnum`: `healthy`, `unhealthy.draining`, ...
    pub state: String,
    pub reason: String,
    pub description: String,
    pub administrative_override: String,
    pub anomaly: String,
}

/// One CloudWatch chart (last 3 hours, 5-minute period).
#[derive(Clone, Debug, PartialEq)]
pub struct Metric {
    pub name: &'static str,
    pub unit: &'static str,
    pub points: Vec<(i64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadBalancerDetail {
    pub load_balancer: LoadBalancer,
    pub listeners: Section<Listener>,
    pub resource_map: Section<ResourceRoute>,
    pub security_groups: Section<SecurityGroup>,
    pub monitoring: Section<Metric>,
    /// (key, value) as the API returns them.
    pub attributes: Section<(String, String)>,
    pub tags: Section<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetGroupDetail {
    pub target_group: TargetGroup,
    pub targets: Section<Target>,
    pub monitoring: Section<Metric>,
    pub attributes: Section<(String, String)>,
    pub tags: Section<(String, String)>,
}

/// Monitoring charts by namespace: (metric, statistic, unit), console order.
const ALB_METRICS: &[(&str, Statistic, &str)] = &[
    ("RequestCount", Statistic::Sum, "Count"),
    ("TargetResponseTime", Statistic::Average, "Seconds"),
    ("HTTPCode_Target_2XX_Count", Statistic::Sum, "Count"),
    ("HTTPCode_Target_4XX_Count", Statistic::Sum, "Count"),
    ("HTTPCode_Target_5XX_Count", Statistic::Sum, "Count"),
    ("HTTPCode_ELB_4XX_Count", Statistic::Sum, "Count"),
    ("HTTPCode_ELB_5XX_Count", Statistic::Sum, "Count"),
    ("ActiveConnectionCount", Statistic::Sum, "Count"),
    ("NewConnectionCount", Statistic::Sum, "Count"),
    ("ProcessedBytes", Statistic::Sum, "Bytes"),
    ("ConsumedLCUs", Statistic::Sum, "Count"),
];
const NLB_METRICS: &[(&str, Statistic, &str)] = &[
    ("ActiveFlowCount", Statistic::Average, "Count"),
    ("NewFlowCount", Statistic::Sum, "Count"),
    ("ProcessedBytes", Statistic::Sum, "Bytes"),
    ("ProcessedPackets", Statistic::Sum, "Count"),
    ("TCP_Client_Reset_Count", Statistic::Sum, "Count"),
    ("TCP_Target_Reset_Count", Statistic::Sum, "Count"),
    ("TCP_ELB_Reset_Count", Statistic::Sum, "Count"),
    ("ConsumedLCUs", Statistic::Sum, "Count"),
];
const GWLB_METRICS: &[(&str, Statistic, &str)] = &[
    ("ActiveFlowCount", Statistic::Average, "Count"),
    ("NewFlowCount", Statistic::Sum, "Count"),
    ("ProcessedBytes", Statistic::Sum, "Bytes"),
    ("ConsumedLCUs", Statistic::Sum, "Count"),
];
const ALB_TARGET_GROUP_METRICS: &[(&str, Statistic, &str)] = &[
    ("HealthyHostCount", Statistic::Average, "Count"),
    ("UnHealthyHostCount", Statistic::Average, "Count"),
    ("RequestCount", Statistic::Sum, "Count"),
    ("TargetResponseTime", Statistic::Average, "Seconds"),
    ("HTTPCode_Target_2XX_Count", Statistic::Sum, "Count"),
    ("HTTPCode_Target_4XX_Count", Statistic::Sum, "Count"),
    ("HTTPCode_Target_5XX_Count", Statistic::Sum, "Count"),
];
const FLOW_TARGET_GROUP_METRICS: &[(&str, Statistic, &str)] = &[
    ("HealthyHostCount", Statistic::Average, "Count"),
    ("UnHealthyHostCount", Statistic::Average, "Count"),
];

/// The most charts any Monitoring tab shows; bounds its scroll.
pub const MAX_CHARTS: usize = ALB_METRICS.len();

fn text(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn iso(value: Option<&aws_smithy_types::DateTime>) -> String {
    value
        .and_then(|t| t.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

/// The resource name inside an ELB ARN: `.../app/web/50dc6c49` -> `web`.
pub fn name_from_arn(arn: &str) -> String {
    let path: Vec<&str> = arn
        .rsplit_once(':')
        .map_or("", |(_, p)| p)
        .split('/')
        .collect();
    match path.as_slice() {
        ["loadbalancer", _, name, _] | ["targetgroup", name, _] => (*name).to_string(),
        _ => String::new(),
    }
}

/// The CloudWatch dimension value of an ELB ARN: `app/web/50dc6c49` or
/// `targetgroup/web/73e2d6bc`.
pub fn dimension(arn: &str) -> String {
    let path = arn.rsplit_once(':').map_or("", |(_, p)| p);
    path.strip_prefix("loadbalancer/")
        .unwrap_or(path)
        .to_string()
}

/// `arn:<partition>:elasticloadbalancing:<region>:<account>:<kind>/...`,
/// in the active Region and account.
fn validate_arn(config: &AwsConfig, arn: &str, kind: &str) -> Result<()> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let well_formed = parts.len() == 6
        && parts[0] == "arn"
        && !parts[1].is_empty()
        && parts[2] == "elasticloadbalancing"
        && !parts[3].is_empty()
        && parts[4].len() == 12
        && parts[4].bytes().all(|c| c.is_ascii_digit())
        && parts[5].starts_with(&format!("{kind}/"))
        && !name_from_arn(arn).is_empty()
        && !arn.chars().any(char::is_control);
    if !well_formed {
        bail!("not an Elastic Load Balancing {kind} ARN: {arn:?}");
    }
    if !config.region.is_empty() && parts[3] != config.region {
        bail!(
            "{kind} ARN Region {:?} does not match the active Region {:?}",
            parts[3],
            config.region
        );
    }
    if !config.account_id.is_empty() && parts[4] != config.account_id {
        bail!(
            "{kind} ARN account {:?} does not match the active account {:?}",
            parts[4],
            config.account_id
        );
    }
    Ok(())
}

async fn bounded<T>(what: &str, read: impl Future<Output = Result<T>>) -> Result<T, String> {
    match tokio::time::timeout(ENRICH_TIMEOUT, read).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => {
            tracing::warn!("ELB {what} unavailable: {error:#}");
            Err(format!("{what} unavailable: {error:#}"))
        }
        Err(_) => Err(format!(
            "{what} unavailable: timed out after {ENRICH_TIMEOUT:?}"
        )),
    }
}

pub(crate) fn load_balancer(lb: &SdkLoadBalancer) -> LoadBalancer {
    LoadBalancer {
        name: text(lb.load_balancer_name()),
        arn: text(lb.load_balancer_arn()),
        dns_name: text(lb.dns_name()),
        state: lb
            .state()
            .and_then(|s| s.code())
            .map(|c| c.as_str().to_string())
            .unwrap_or_default(),
        state_reason: text(lb.state().and_then(|s| s.reason())),
        lb_type: lb
            .r#type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        scheme: lb
            .scheme()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        vpc_id: text(lb.vpc_id()),
        zones: lb
            .availability_zones()
            .iter()
            .map(|z| {
                let addresses = z.load_balancer_addresses();
                let pick = |f: fn(
                    &aws_sdk_elasticloadbalancingv2::types::LoadBalancerAddress,
                ) -> Option<&str>| {
                    addresses.iter().filter_map(f).map(str::to_string).collect()
                };
                ZoneMapping {
                    zone: text(z.zone_name()),
                    subnet_id: text(z.subnet_id()),
                    ipv4: pick(|a| a.ip_address()),
                    private_ipv4: pick(|a| a.private_ipv4_address()),
                    ipv6: pick(|a| a.ipv6_address()),
                }
            })
            .collect(),
        ip_address_type: lb
            .ip_address_type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        created_time: iso(lb.created_time()),
        security_groups: lb.security_groups().to_vec(),
        hosted_zone: text(lb.canonical_hosted_zone_id()),
    }
}

pub(crate) fn target_group(tg: &SdkTargetGroup) -> TargetGroup {
    let raw = |v: Option<&aws_sdk_elasticloadbalancingv2::types::ProtocolEnum>| {
        v.map(|p| p.as_str().to_string()).unwrap_or_default()
    };
    TargetGroup {
        name: text(tg.target_group_name()),
        arn: text(tg.target_group_arn()),
        port: tg.port(),
        protocol: raw(tg.protocol()),
        protocol_version: text(tg.protocol_version()),
        target_type: tg
            .target_type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        vpc_id: text(tg.vpc_id()),
        ip_address_type: tg
            .ip_address_type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        load_balancer_arns: tg.load_balancer_arns().to_vec(),
        health_check: HealthCheck {
            enabled: tg.health_check_enabled(),
            protocol: raw(tg.health_check_protocol()),
            port: text(tg.health_check_port()),
            path: text(tg.health_check_path()),
            healthy_threshold: tg.healthy_threshold_count(),
            unhealthy_threshold: tg.unhealthy_threshold_count(),
            timeout_seconds: tg.health_check_timeout_seconds(),
            interval_seconds: tg.health_check_interval_seconds(),
            success_codes: tg
                .matcher()
                .and_then(|m| m.http_code().or(m.grpc_code()))
                .unwrap_or_default()
                .to_string(),
        },
    }
}

/// The console wording of a listener action. `names` resolves target group
/// ARNs to names.
pub(crate) fn action(a: &Action, names: &dyn Fn(&str) -> String) -> String {
    match a.r#type() {
        Some(ActionTypeEnum::Forward) => {
            let groups: Vec<_> = match a.forward_config() {
                Some(config) if !config.target_groups().is_empty() => {
                    let tuples = config.target_groups();
                    let total: i32 = tuples.iter().map(|t| t.weight().unwrap_or(1)).sum();
                    tuples
                        .iter()
                        .filter_map(|t| {
                            let share = t.weight().unwrap_or(1) * 100 / total.max(1);
                            Some(format!("{} ({share}%)", names(t.target_group_arn()?)))
                        })
                        .collect()
                }
                _ => a
                    .target_group_arn()
                    .map(|arn| format!("{} (100%)", names(arn)))
                    .into_iter()
                    .collect(),
            };
            format!("Forward to target group: {}", groups.join(", "))
        }
        Some(ActionTypeEnum::Redirect) => {
            let r = a.redirect_config();
            let part = |v: Option<&str>, default: &str| v.unwrap_or(default).to_string();
            format!(
                "Redirect to {}://{}:{}{}?{}",
                part(r.and_then(|r| r.protocol()), "#{protocol}"),
                part(r.and_then(|r| r.host()), "#{host}"),
                part(r.and_then(|r| r.port()), "#{port}"),
                part(r.and_then(|r| r.path()), "/#{path}"),
                part(r.and_then(|r| r.query()), "#{query}"),
            )
        }
        Some(ActionTypeEnum::FixedResponse) => format!(
            "Return fixed response: {}",
            text(a.fixed_response_config().and_then(|f| f.status_code()))
        ),
        Some(ActionTypeEnum::AuthenticateOidc) => "Authenticate (OIDC)".into(),
        Some(ActionTypeEnum::AuthenticateCognito) => "Authenticate (Amazon Cognito)".into(),
        Some(ActionTypeEnum::JwtValidation) => "Validate JWT".into(),
        Some(other) => other.as_str().to_string(),
        None => String::new(),
    }
}

/// Target group ARNs an action forwards to.
fn forwards(a: &Action) -> Vec<String> {
    let mut arns: Vec<String> = a
        .forward_config()
        .map(|c| {
            c.target_groups()
                .iter()
                .filter_map(|t| t.target_group_arn())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if arns.is_empty() {
        arns.extend(a.target_group_arn().map(str::to_string));
    }
    arns
}

/// Actions in order, the terminal action last.
fn ordered(actions: &[Action]) -> Vec<&Action> {
    let mut actions: Vec<&Action> = actions.iter().collect();
    actions.sort_by_key(|a| a.order().unwrap_or(i32::MAX));
    actions
}

pub(crate) fn listener(l: &SdkListener, names: &dyn Fn(&str) -> String) -> Listener {
    let actions = ordered(l.default_actions());
    Listener {
        arn: text(l.listener_arn()),
        protocol: l
            .protocol()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default(),
        port: l.port(),
        default_action: actions.last().map(|a| action(a, names)).unwrap_or_default(),
        rules: None,
        ssl_policy: text(l.ssl_policy()),
        default_certificate: text(l.certificates().first().and_then(|c| c.certificate_arn())),
        mtls_mode: text(l.mutual_authentication().and_then(|m| m.mode())),
        trust_store_arn: text(l.mutual_authentication().and_then(|m| m.trust_store_arn())),
    }
}

/// Health counts of each target group, by ARN.
pub(crate) fn health_counts(targets: &[TargetHealthDescription]) -> (usize, usize, usize) {
    let state = |t: &&TargetHealthDescription| {
        t.target_health()
            .and_then(|h| h.state())
            .map(|s| s.as_str().to_string())
            .unwrap_or_default()
    };
    let healthy = targets.iter().filter(|t| state(t) == "healthy").count();
    let unhealthy = targets
        .iter()
        .filter(|t| state(t).starts_with("unhealthy"))
        .count();
    (targets.len(), healthy, unhealthy)
}

pub(crate) fn target(t: &TargetHealthDescription) -> Target {
    let description = t.target();
    let health = t.target_health();
    Target {
        id: text(description.and_then(|d| d.id())),
        name: String::new(),
        port: description.and_then(|d| d.port()),
        zone: text(description.and_then(|d| d.availability_zone())),
        state: health
            .and_then(|h| h.state())
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        reason: health
            .and_then(|h| h.reason())
            .map(|r| r.as_str().to_string())
            .unwrap_or_default(),
        description: text(health.and_then(|h| h.description())),
        administrative_override: t
            .administrative_override()
            .and_then(|o| o.state())
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        anomaly: t
            .anomaly_detection()
            .and_then(|a| a.result())
            .map(|r| r.as_str().to_string())
            .unwrap_or_default(),
    }
}

/// (key, value) pairs, as the API returns them.
fn pairs<'a>(
    items: impl Iterator<Item = (Option<&'a str>, Option<&'a str>)>,
) -> Vec<(String, String)> {
    items.map(|(k, v)| (text(k), text(v))).collect()
}

pub struct ElbClient {
    config: AwsConfig,
}

impl ElbClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_load_balancers(&self) -> Result<Vec<LoadBalancer>> {
        let pages = self
            .config
            .elbv2_client()
            .describe_load_balancers()
            .into_paginator()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await
            .context("ELB DescribeLoadBalancers request failed")?;
        let mut rows: Vec<_> = pages
            .iter()
            .flat_map(|p| p.load_balancers())
            .map(load_balancer)
            .collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(rows)
    }

    pub async fn list_target_groups(&self) -> Result<Vec<TargetGroup>> {
        let pages = self
            .config
            .elbv2_client()
            .describe_target_groups()
            .into_paginator()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await
            .context("ELB DescribeTargetGroups request failed")?;
        let mut rows: Vec<_> = pages
            .iter()
            .flat_map(|p| p.target_groups())
            .map(target_group)
            .collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(rows)
    }

    pub async fn load_balancer_details(&self, arn: &str) -> Result<LoadBalancerDetail> {
        validate_arn(&self.config, arn, "loadbalancer")?;
        let client = self.config.elbv2_client();
        let out = tokio::time::timeout(
            TIMEOUT,
            client
                .describe_load_balancers()
                .load_balancer_arns(arn)
                .send(),
        )
        .await
        .map_err(|_| anyhow!("ELB DescribeLoadBalancers timed out after {TIMEOUT:?}"))?
        .context("ELB DescribeLoadBalancers request failed")?;
        let found = out.load_balancers();
        let [lb] = found else {
            bail!(
                "ELB DescribeLoadBalancers returned {} load balancers for {arn:?}; expected exactly one",
                found.len()
            );
        };
        if lb.load_balancer_arn() != Some(arn) {
            bail!(
                "ELB DescribeLoadBalancers returned {:?} for requested ARN {arn:?}; refusing to \
                 present a mismatched identity",
                lb.load_balancer_arn().unwrap_or_default()
            );
        }
        let lb = load_balancer(lb);
        let (map, security_groups, monitoring, attributes, tags) = tokio::join!(
            self.listeners_and_routes(&lb),
            bounded("Security groups", self.security_groups(&lb.security_groups)),
            bounded("CloudWatch metrics", self.load_balancer_metrics(&lb)),
            bounded("Attributes", async {
                let out = client
                    .describe_load_balancer_attributes()
                    .load_balancer_arn(arn)
                    .send()
                    .await?;
                Ok(pairs(out.attributes().iter().map(|a| (a.key(), a.value()))))
            }),
            bounded("Tags", self.tags(arn)),
        );
        let (listeners, resource_map) = map;
        Ok(LoadBalancerDetail {
            load_balancer: lb,
            listeners,
            resource_map,
            security_groups,
            monitoring,
            attributes,
            tags,
        })
    }

    pub async fn target_group_details(&self, arn: &str) -> Result<TargetGroupDetail> {
        validate_arn(&self.config, arn, "targetgroup")?;
        let client = self.config.elbv2_client();
        let out = tokio::time::timeout(
            TIMEOUT,
            client
                .describe_target_groups()
                .target_group_arns(arn)
                .send(),
        )
        .await
        .map_err(|_| anyhow!("ELB DescribeTargetGroups timed out after {TIMEOUT:?}"))?
        .context("ELB DescribeTargetGroups request failed")?;
        let found = out.target_groups();
        let [tg] = found else {
            bail!(
                "ELB DescribeTargetGroups returned {} target groups for {arn:?}; expected exactly one",
                found.len()
            );
        };
        if tg.target_group_arn() != Some(arn) {
            bail!(
                "ELB DescribeTargetGroups returned {:?} for requested ARN {arn:?}; refusing to \
                 present a mismatched identity",
                tg.target_group_arn().unwrap_or_default()
            );
        }
        let tg = target_group(tg);
        let (targets, monitoring, attributes, tags) = tokio::join!(
            bounded("Targets", self.targets(&tg)),
            bounded("CloudWatch metrics", self.target_group_metrics(&tg)),
            bounded("Attributes", async {
                let out = client
                    .describe_target_group_attributes()
                    .target_group_arn(arn)
                    .send()
                    .await?;
                Ok(pairs(out.attributes().iter().map(|a| (a.key(), a.value()))))
            }),
            bounded("Tags", self.tags(arn)),
        );
        Ok(TargetGroupDetail {
            target_group: tg,
            targets,
            monitoring,
            attributes,
            tags,
        })
    }

    async fn tags(&self, arn: &str) -> Result<Vec<(String, String)>> {
        let out = self
            .config
            .elbv2_client()
            .describe_tags()
            .resource_arns(arn)
            .send()
            .await?;
        let mut tags: Vec<_> = out
            .tag_descriptions()
            .iter()
            .filter(|d| d.resource_arn() == Some(arn))
            .flat_map(|d| pairs(d.tags().iter().map(|t| (t.key(), t.value()))))
            .collect();
        tags.sort();
        Ok(tags)
    }

    async fn security_groups(&self, ids: &[String]) -> Result<Vec<SecurityGroup>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let out = self
            .config
            .ec2_client()
            .describe_security_groups()
            .set_group_ids(Some(ids.to_vec()))
            .send()
            .await?;
        let mut rows: Vec<_> = out
            .security_groups()
            .iter()
            .filter(|g| {
                g.group_id()
                    .is_some_and(|id| ids.iter().any(|own| own == id))
            })
            .map(|g| SecurityGroup {
                id: text(g.group_id()),
                name: text(g.group_name()),
                description: text(g.description()),
            })
            .collect();
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(rows)
    }

    /// Listeners (with rule counts) and the Resource map routes. The routes
    /// need the listeners, so both fail together when DescribeListeners does.
    async fn listeners_and_routes(
        &self,
        lb: &LoadBalancer,
    ) -> (Section<Listener>, Section<ResourceRoute>) {
        let client = self.config.elbv2_client();
        let read = async {
            let pages = client
                .describe_listeners()
                .load_balancer_arn(&lb.arn)
                .into_paginator()
                .send()
                .collect::<Result<Vec<_>, _>>()
                .await?;
            let listeners: Vec<SdkListener> =
                pages.iter().flat_map(|p| p.listeners().to_vec()).collect();
            let groups = client
                .describe_target_groups()
                .load_balancer_arn(&lb.arn)
                .send()
                .await?
                .target_groups()
                .to_vec();
            Ok::<_, anyhow::Error>((listeners, groups))
        };
        let (listeners, groups) = match bounded("Listeners", read).await {
            Ok(found) => found,
            Err(error) => return (Err(error.clone()), Err(error)),
        };
        let names = |arn: &str| {
            groups
                .iter()
                .find(|g| g.target_group_arn() == Some(arn))
                .and_then(|g| g.target_group_name())
                .map(str::to_string)
                .unwrap_or_else(|| name_from_arn(arn))
        };
        // Rules only exist on Application Load Balancer listeners.
        let with_rules = lb.lb_type == "application";
        let rule_reads = listeners.iter().map(|l| {
            let client = &client;
            let arn = text(l.listener_arn());
            async move {
                if !with_rules {
                    return Ok(Vec::new());
                }
                bounded("Rules", async {
                    let pages = client
                        .describe_rules()
                        .listener_arn(&arn)
                        .into_paginator()
                        .send()
                        .collect::<Result<Vec<_>, _>>()
                        .await?;
                    Ok(pages.iter().flat_map(|p| p.rules().to_vec()).collect())
                })
                .await
            }
        });
        let health_reads = groups.iter().map(|g| {
            let client = &client;
            let arn = text(g.target_group_arn());
            async move {
                let read = async {
                    Ok(client
                        .describe_target_health()
                        .target_group_arn(&arn)
                        .send()
                        .await?
                        .target_health_descriptions()
                        .to_vec())
                };
                let health = bounded("Target health", read).await.ok();
                (arn, health)
            }
        });
        let (rules, health) = tokio::join!(
            futures::future::join_all(rule_reads),
            futures::future::join_all(health_reads),
        );
        let counts = |arn: &str| {
            health
                .iter()
                .find(|(a, _)| a == arn)
                .and_then(|(_, h)| h.as_deref())
                .map(health_counts)
        };
        let mut rows = Vec::new();
        let mut routes = Vec::new();
        for (l, rules) in listeners.iter().zip(rules) {
            let mut row = listener(l, &names);
            let label = format!(
                "{}:{}",
                row.protocol,
                row.port.map(|p| p.to_string()).unwrap_or_default()
            );
            // (rule label, actions) per route; the default rule from the
            // listener when the rules are not read.
            let mut by_rule: Vec<(String, Vec<&Action>)> = Vec::new();
            match &rules {
                Ok(rules) if with_rules => {
                    row.rules = Some(rules.len());
                    for r in rules {
                        let rule = if r.is_default() == Some(true) {
                            "Default".to_string()
                        } else {
                            format!("Priority {}", text(r.priority()))
                        };
                        by_rule.push((rule, ordered(r.actions())));
                    }
                }
                _ => by_rule.push(("Default".into(), ordered(l.default_actions()))),
            }
            for (rule, actions) in by_rule {
                for arn in actions.last().map(|a| forwards(a)).unwrap_or_default() {
                    let health = counts(&arn);
                    routes.push(ResourceRoute {
                        listener: label.clone(),
                        rule: rule.clone(),
                        target_group: names(&arn),
                        targets: health.map(|h| h.0),
                        healthy: health.map(|h| h.1),
                        unhealthy: health.map(|h| h.2),
                        target_group_arn: arn,
                    });
                }
            }
            rows.push(row);
        }
        rows.sort_by_key(|l| l.port);
        (Ok(rows), Ok(routes))
    }

    async fn targets(&self, tg: &TargetGroup) -> Result<Vec<Target>> {
        let out = self
            .config
            .elbv2_client()
            .describe_target_health()
            .target_group_arn(&tg.arn)
            .send()
            .await?;
        let mut rows: Vec<Target> = out
            .target_health_descriptions()
            .iter()
            .map(target)
            .collect();
        if tg.target_type == "instance" && !rows.is_empty() {
            // Names are optional: a failed instance read leaves them empty.
            let ids: Vec<String> = rows.iter().map(|t| t.id.clone()).collect();
            let names = bounded("Instance names", async {
                let pages = self
                    .config
                    .ec2_client()
                    .describe_instances()
                    .set_instance_ids(Some(ids))
                    .send()
                    .await?;
                Ok(pages
                    .reservations()
                    .iter()
                    .flat_map(|r| r.instances())
                    .filter_map(|i| {
                        let name = i.tags().iter().find(|t| t.key() == Some("Name"))?.value()?;
                        Some((i.instance_id()?.to_string(), name.to_string()))
                    })
                    .collect::<Vec<_>>())
            })
            .await
            .unwrap_or_default();
            for row in &mut rows {
                if let Some((_, name)) = names.iter().find(|(id, _)| *id == row.id) {
                    row.name = name.clone();
                }
            }
        }
        rows.sort_by(|a, b| (&a.id, a.port).cmp(&(&b.id, b.port)));
        Ok(rows)
    }

    async fn metrics(
        &self,
        namespace: &str,
        specs: &'static [(&'static str, Statistic, &'static str)],
        dimensions: &[(&str, String)],
    ) -> Result<Vec<Metric>> {
        let reader = MetricReader::new(self.config.cloudwatch_client());
        let reads = specs.iter().map(|(name, statistic, unit)| {
            let reader = &reader;
            async move {
                let mut query = MetricQuery::new(namespace, name, statistic.clone());
                for (key, value) in dimensions {
                    query = query.dimension(key, value);
                }
                Ok::<_, anyhow::Error>(Metric {
                    name,
                    unit,
                    points: reader.recent(query, 300).await?,
                })
            }
        });
        futures::future::try_join_all(reads).await
    }

    async fn load_balancer_metrics(&self, lb: &LoadBalancer) -> Result<Vec<Metric>> {
        let (namespace, specs) = match lb.lb_type.as_str() {
            "application" => ("AWS/ApplicationELB", ALB_METRICS),
            "network" => ("AWS/NetworkELB", NLB_METRICS),
            "gateway" => ("AWS/GatewayELB", GWLB_METRICS),
            other => bail!("no metrics for load balancer type {other:?}"),
        };
        self.metrics(namespace, specs, &[("LoadBalancer", dimension(&lb.arn))])
            .await
    }

    /// Charts for the target group behind its first load balancer, as the
    /// console scopes them.
    async fn target_group_metrics(&self, tg: &TargetGroup) -> Result<Vec<Metric>> {
        let Some(lb) = tg.load_balancer_arns.first() else {
            bail!("the target group is not associated with a load balancer");
        };
        let lb_dimension = dimension(lb);
        let (namespace, specs) = match lb_dimension.split('/').next() {
            Some("app") => ("AWS/ApplicationELB", ALB_TARGET_GROUP_METRICS),
            Some("net") => ("AWS/NetworkELB", FLOW_TARGET_GROUP_METRICS),
            Some("gwy") => ("AWS/GatewayELB", FLOW_TARGET_GROUP_METRICS),
            _ => bail!("unknown load balancer ARN {lb:?}"),
        };
        self.metrics(
            namespace,
            specs,
            &[
                ("TargetGroup", dimension(&tg.arn)),
                ("LoadBalancer", lb_dimension),
            ],
        )
        .await
    }
}

#[cfg(test)]
mod tests;
