//! Read-only EventBridge inventory: event buses, the rules on one bus, and one
//! rule with its targets, tags and CloudWatch metrics. ListEventBuses,
//! Schemas ListDiscoverers, ListRules, DescribeRule, ListTargetsByRule,
//! ListTagsForResource and GetMetricStatistics only; never a put, delete,
//! enable or disable call.
//!
//! ListEventBuses returns Custom Event Bus - Classic buses only. The newer
//! Custom Event Bus (EventBridgeV2, `event-busv2` ARNs) is not listed.
use crate::config::AwsConfig;
use crate::metrics::{MetricQuery, MetricReader};
use crate::pagination::{Pagination, StopReason};
use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_cloudwatch::types::Statistic;
use aws_smithy_types::date_time::Format;
use aws_smithy_types::DateTime;
use std::future::Future;
use std::time::Duration;

const LIST_TIMEOUT: Duration = Duration::from_secs(30);
/// Bound on each optional read after DescribeRule succeeded.
const SECTION_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_PAGES: usize = 1000;

/// One table of the rule detail: its rows, or why they are unavailable.
pub type Section<T> = Result<Vec<T>, String>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventBus {
    pub name: String,
    pub arn: String,
    pub description: String,
    /// RFC3339 UTC, or empty.
    pub created: String,
    pub last_modified: String,
    /// Raw schema discoverer state for this bus (`STARTED`, `STOPPED`), empty
    /// when it has none, "Unavailable" when the Schemas read failed.
    pub schema_discovery: String,
}

impl EventBus {
    /// Console type of a bus from ListEventBuses.
    pub fn bus_type(&self) -> &'static str {
        if self.name == "default" {
            "Default"
        } else if self.name.starts_with("aws.partner/") {
            "Partner"
        } else {
            "Custom Event Bus - Classic"
        }
    }

    /// The account segment of the ARN.
    pub fn owner_account(&self) -> &str {
        self.arn.split(':').nth(4).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventRule {
    pub name: String,
    pub arn: String,
    /// Raw `RuleState`: ENABLED, DISABLED, ENABLED_WITH_ALL_CLOUDTRAIL_MANAGEMENT_EVENTS.
    pub state: String,
    pub description: String,
    pub event_pattern: String,
    pub schedule_expression: String,
    pub role_arn: String,
    pub managed_by: String,
    pub event_bus_name: String,
}

impl EventRule {
    /// Console type: a schedule expression makes a rule scheduled.
    pub fn rule_type(&self) -> &'static str {
        if self.schedule_expression.is_empty() {
            "Standard"
        } else {
            "Scheduled"
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventTarget {
    pub id: String,
    pub arn: String,
    pub role_arn: String,
    /// Input, input path or input transformer template, as configured.
    pub input: String,
}

/// One Monitoring chart (last 3 hours, 5-minute sums).
#[derive(Clone, Debug, PartialEq)]
pub struct RuleMetric {
    pub name: &'static str,
    pub points: Vec<(i64, f64)>,
}

/// Monitoring charts, in console order.
pub const RULE_METRICS: &[&str] = &[
    "Invocations",
    "FailedInvocations",
    "TriggeredRules",
    "MatchedEvents",
    "ThrottledRules",
];

#[derive(Clone, Debug, PartialEq)]
pub struct RuleDetail {
    pub rule: EventRule,
    pub created_by: String,
    pub targets: Section<EventTarget>,
    pub tags: Section<(String, String)>,
    pub metrics: Section<RuleMetric>,
}

fn rfc3339(ts: Option<&DateTime>) -> String {
    ts.and_then(|dt| dt.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

fn text(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

/// `arn:<partition>:events:<region>:<account>:rule/[<bus>/]<name>`; returns
/// (bus, name). Rules on the default bus have no bus segment.
pub fn rule_identity(arn: &str) -> Result<(String, String)> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let resource = match parts.as_slice() {
        ["arn", partition, "events", region, account, resource]
            if !partition.is_empty()
                && !region.is_empty()
                && account.len() == 12
                && account.bytes().all(|c| c.is_ascii_digit())
                && !arn.chars().any(char::is_control) =>
        {
            resource.strip_prefix("rule/")
        }
        _ => None,
    };
    match resource
        .map(|r| r.split('/').collect::<Vec<_>>())
        .as_deref()
    {
        Some([name]) if !name.is_empty() => Ok(("default".into(), name.to_string())),
        Some([bus, name]) if !bus.is_empty() && !name.is_empty() => {
            Ok((bus.to_string(), name.to_string()))
        }
        _ => bail!("not an EventBridge rule ARN: {arn:?}"),
    }
}

/// The discoverer state per bus ARN; "Unavailable" when the read failed.
pub(crate) fn apply_discoverers(buses: &mut [EventBus], discoverers: Option<&[(String, String)]>) {
    for bus in buses {
        bus.schema_discovery = match discoverers {
            None => "Unavailable".into(),
            Some(rows) => rows
                .iter()
                .find(|(source, _)| *source == bus.arn)
                .map(|(_, state)| state.clone())
                .unwrap_or_default(),
        };
    }
}

fn rule(r: &aws_sdk_eventbridge::types::Rule) -> EventRule {
    EventRule {
        name: text(r.name()),
        arn: text(r.arn()),
        state: text(r.state().map(|s| s.as_str())),
        description: text(r.description()),
        event_pattern: text(r.event_pattern()),
        schedule_expression: text(r.schedule_expression()),
        role_arn: text(r.role_arn()),
        managed_by: text(r.managed_by()),
        event_bus_name: text(r.event_bus_name()),
    }
}

fn target(t: &aws_sdk_eventbridge::types::Target) -> EventTarget {
    let input = match (t.input(), t.input_path(), t.input_transformer()) {
        (Some(input), _, _) => input.to_string(),
        (_, Some(path), _) => path.to_string(),
        (_, _, Some(transformer)) => transformer.input_template().to_string(),
        _ => String::new(),
    };
    EventTarget {
        id: t.id().into(),
        arn: t.arn().into(),
        role_arn: text(t.role_arn()),
        input,
    }
}

async fn bounded<T>(what: &str, read: impl Future<Output = Result<Vec<T>>>) -> Section<T> {
    match tokio::time::timeout(SECTION_TIMEOUT, read).await {
        Ok(Ok(rows)) => Ok(rows),
        Ok(Err(error)) => Err(format!("{what} unavailable: {error:#}")),
        Err(_) => Err(format!(
            "{what} unavailable: timed out after {SECTION_TIMEOUT:?}"
        )),
    }
}

fn repeated(api: &str) -> anyhow::Error {
    anyhow!("EventBridge {api} returned a repeated pagination token; aborting")
}

pub struct EventBridgeClient {
    config: AwsConfig,
}

impl EventBridgeClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    /// Every event bus in the Region, `default` first, then by name.
    /// Schema discovery comes from a parallel, bounded Schemas read; its
    /// failure marks only that column.
    pub async fn list_event_buses(&self) -> Result<Vec<EventBus>> {
        let (buses, discoverers) = tokio::join!(
            tokio::time::timeout(LIST_TIMEOUT, self.buses_inner()),
            tokio::time::timeout(SECTION_TIMEOUT, self.discoverers()),
        );
        let mut buses = buses.map_err(|_| {
            anyhow!("EventBridge ListEventBuses timed out after {LIST_TIMEOUT:?}")
        })??;
        let discoverers = match discoverers {
            Ok(Ok(rows)) => Some(rows),
            Ok(Err(error)) => {
                tracing::warn!("EventBridge schema discoverers unavailable: {error:#}");
                None
            }
            Err(_) => None,
        };
        apply_discoverers(&mut buses, discoverers.as_deref());
        Ok(buses)
    }

    /// (source ARN, state) of every schema discoverer.
    async fn discoverers(&self) -> Result<Vec<(String, String)>> {
        let pages = self
            .config
            .schemas_client()
            .list_discoverers()
            .into_paginator()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?;
        Ok(pages
            .iter()
            .flat_map(|p| p.discoverers())
            .filter_map(|d| {
                Some((
                    d.source_arn()?.to_string(),
                    d.state()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default(),
                ))
            })
            .collect())
    }

    async fn buses_inner(&self) -> Result<Vec<EventBus>> {
        let client = self.config.eventbridge_client();
        let mut buses = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(MAX_PAGES);
        loop {
            let page = client
                .list_event_buses()
                .set_next_token(token)
                .send()
                .await
                .context("EventBridge ListEventBuses request failed")?;
            buses.extend(page.event_buses().iter().map(|b| EventBus {
                name: text(b.name()),
                arn: text(b.arn()),
                description: text(b.description()),
                created: rfc3339(b.creation_time()),
                last_modified: rfc3339(b.last_modified_time()),
                schema_discovery: String::new(),
            }));
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => break,
                Err(StopReason::RepeatedToken) => return Err(repeated("ListEventBuses")),
                Err(_) => bail!("EventBridge ListEventBuses exceeded {MAX_PAGES} pages"),
            }
        }
        buses.sort_by(|a, b| (a.name != "default", &a.name).cmp(&(b.name != "default", &b.name)));
        Ok(buses)
    }

    /// Every rule on one bus, by name. Rules from another bus are refused.
    pub async fn list_rules(&self, bus: &str) -> Result<Vec<EventRule>> {
        tokio::time::timeout(LIST_TIMEOUT, self.rules_inner(bus))
            .await
            .map_err(|_| anyhow!("EventBridge ListRules timed out after {LIST_TIMEOUT:?}"))?
    }

    async fn rules_inner(&self, bus: &str) -> Result<Vec<EventRule>> {
        if bus.is_empty() {
            bail!("no event bus");
        }
        let client = self.config.eventbridge_client();
        let mut rules = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(MAX_PAGES);
        loop {
            let page = client
                .list_rules()
                .event_bus_name(bus)
                .set_next_token(token)
                .send()
                .await
                .context("EventBridge ListRules request failed")?;
            rules.extend(page.rules().iter().map(rule));
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => break,
                Err(StopReason::RepeatedToken) => return Err(repeated("ListRules")),
                Err(_) => bail!("EventBridge ListRules exceeded {MAX_PAGES} pages"),
            }
        }
        if let Some(other) = rules
            .iter()
            .find(|r| !r.event_bus_name.is_empty() && r.event_bus_name != bus)
        {
            bail!(
                "EventBridge ListRules for bus {bus:?} returned rule {:?} on bus {:?}",
                other.name,
                other.event_bus_name
            );
        }
        rules.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(rules)
    }

    /// One rule by ARN: DescribeRule (identity verified), then targets, tags
    /// and metrics in parallel, each bounded and failing on its own.
    pub async fn rule_details(&self, arn: &str) -> Result<RuleDetail> {
        let (bus, name) = rule_identity(arn)?;
        let parts: Vec<_> = arn.splitn(6, ':').collect();
        if !self.config.region.is_empty() && parts[3] != self.config.region {
            bail!(
                "rule ARN Region {:?} does not match the active Region {:?}",
                parts[3],
                self.config.region
            );
        }
        if !self.config.account_id.is_empty() && parts[4] != self.config.account_id {
            bail!(
                "rule ARN account {:?} does not match the active account {:?}",
                parts[4],
                self.config.account_id
            );
        }
        let client = self.config.eventbridge_client();
        let out = tokio::time::timeout(
            LIST_TIMEOUT,
            client
                .describe_rule()
                .name(&name)
                .event_bus_name(&bus)
                .send(),
        )
        .await
        .map_err(|_| anyhow!("EventBridge DescribeRule timed out after {LIST_TIMEOUT:?}"))?
        .context("EventBridge DescribeRule request failed")?;
        if out.arn() != Some(arn) {
            bail!(
                "EventBridge DescribeRule returned {:?} for requested ARN {arn:?}; refusing \
                 to present a mismatched identity",
                out.arn().unwrap_or_default()
            );
        }
        let described = EventRule {
            name: text(out.name()),
            arn: text(out.arn()),
            state: text(out.state().map(|s| s.as_str())),
            description: text(out.description()),
            event_pattern: text(out.event_pattern()),
            schedule_expression: text(out.schedule_expression()),
            role_arn: text(out.role_arn()),
            managed_by: text(out.managed_by()),
            event_bus_name: text(out.event_bus_name()),
        };
        let (targets, tags, metrics) = tokio::join!(
            bounded("Targets", self.targets(&bus, &name)),
            bounded("Tags", self.tags(arn)),
            bounded("CloudWatch metrics", self.metrics(&bus, &name)),
        );
        Ok(RuleDetail {
            rule: described,
            created_by: text(out.created_by()),
            targets,
            tags,
            metrics,
        })
    }

    async fn targets(&self, bus: &str, name: &str) -> Result<Vec<EventTarget>> {
        let client = self.config.eventbridge_client();
        let mut targets = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(MAX_PAGES);
        loop {
            let page = client
                .list_targets_by_rule()
                .rule(name)
                .event_bus_name(bus)
                .set_next_token(token)
                .send()
                .await?;
            targets.extend(page.targets().iter().map(target));
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => break,
                Err(StopReason::RepeatedToken) => return Err(repeated("ListTargetsByRule")),
                Err(_) => bail!("EventBridge ListTargetsByRule exceeded {MAX_PAGES} pages"),
            }
        }
        targets.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(targets)
    }

    async fn tags(&self, arn: &str) -> Result<Vec<(String, String)>> {
        let out = self
            .config
            .eventbridge_client()
            .list_tags_for_resource()
            .resource_arn(arn)
            .send()
            .await?;
        let mut tags: Vec<_> = out
            .tags()
            .iter()
            .map(|t| (t.key().to_string(), t.value().to_string()))
            .collect();
        tags.sort();
        Ok(tags)
    }

    async fn metrics(&self, bus: &str, name: &str) -> Result<Vec<RuleMetric>> {
        let reader = MetricReader::new(self.config.cloudwatch_client());
        let reads = RULE_METRICS.iter().map(|metric| {
            let reader = &reader;
            async move {
                let mut query = MetricQuery::new("AWS/Events", metric, Statistic::Sum)
                    .dimension("RuleName", name);
                // Rules on a custom bus also carry the bus dimension.
                if bus != "default" {
                    query = query.dimension("EventBusName", bus);
                }
                Ok::<_, anyhow::Error>(RuleMetric {
                    name: metric,
                    points: reader.recent(query, 300).await?,
                })
            }
        });
        futures::future::try_join_all(reads).await
    }
}

#[cfg(test)]
mod tests;
