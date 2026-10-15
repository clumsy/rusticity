//! Cross-service discovery of supported resource metadata, never resource contents.
use crate::config::AwsConfig;
use anyhow::{bail, Result};
use futures::{stream, StreamExt};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, Mutex};

mod inventory;

pub const RESULT_LIMIT: usize = 1000;
const CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceKind {
    Api,
    Stack,
    Alarm,
    LogGroup,
    Instance,
    Vpc,
    Table,
    Repository,
    PublicRepository,
    Efs,
    Fsx,
    User,
    Role,
    Group,
    Key,
    Function,
    Database,
    Bucket,
    Queue,
    /// Generic linked target unsupported as an inventory-search kind.
    /// Related-resource discovery uses this for ARNs that don't map to a
    /// supported top-level inventory kind (e.g. subnets, volumes, layers).
    /// Excluded from `ALL` so inventory search stays at 19 kinds.
    Reference,
}

impl ResourceKind {
    pub const ALL: [Self; 19] = [
        Self::Api,
        Self::Stack,
        Self::Alarm,
        Self::LogGroup,
        Self::Instance,
        Self::Vpc,
        Self::Table,
        Self::Repository,
        Self::PublicRepository,
        Self::Efs,
        Self::Fsx,
        Self::User,
        Self::Role,
        Self::Group,
        Self::Key,
        Self::Function,
        Self::Database,
        Self::Bucket,
        Self::Queue,
    ];
    pub fn service(self) -> &'static str {
        match self {
            Self::Api => "apigateway",
            Self::Stack => "cloudformation",
            Self::Alarm => "cloudwatch",
            Self::LogGroup => "logs",
            Self::Instance => "ec2",
            Self::Vpc => "vpc",
            Self::Table => "dynamodb",
            Self::Repository => "ecr",
            Self::PublicRepository => "ecr-public",
            Self::Efs => "efs",
            Self::Fsx => "fsx",
            Self::User | Self::Role | Self::Group => "iam",
            Self::Key => "kms",
            Self::Function => "lambda",
            Self::Database => "rds",
            Self::Bucket => "s3",
            Self::Queue => "sqs",
            Self::Reference => "",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Api => "API",
            Self::Stack => "Stack",
            Self::Alarm => "Alarm",
            Self::LogGroup => "Log group",
            Self::Instance => "Instance",
            Self::Vpc => "VPC",
            Self::Table => "Table",
            Self::Repository | Self::PublicRepository => "Repository",
            Self::Efs | Self::Fsx => "File system",
            Self::User => "User",
            Self::Role => "Role",
            Self::Group => "Group",
            Self::Key => "Key",
            Self::Function => "Function",
            Self::Database => "Database",
            Self::Bucket => "Bucket",
            Self::Queue => "Queue",
            Self::Reference => "Reference",
        }
    }
    fn resource_types(self) -> &'static str {
        match self {
            Self::Api => "apigateway:restapis,apigateway:apis",
            Self::Stack => "cloudformation:stack",
            Self::Alarm => "cloudwatch:alarm",
            Self::LogGroup => "logs:log-group",
            Self::Instance => "ec2:instance",
            Self::Vpc => "ec2:vpc",
            Self::Table => "dynamodb:table",
            Self::Repository => "ecr:repository",
            Self::PublicRepository => "ecr-public:repository",
            Self::Efs => "elasticfilesystem:file-system",
            Self::Fsx => "fsx:file-system",
            Self::User => "iam:user",
            Self::Role => "iam:role",
            Self::Group => "iam:group",
            Self::Key => "kms:key",
            Self::Function => "lambda:function",
            Self::Database => "rds:db",
            Self::Bucket => "s3:bucket",
            Self::Queue => "sqs:queue",
            Self::Reference => "",
        }
    }
    fn accepts_qualifier(self, qualifier: &str) -> bool {
        if self == Self::Reference {
            return false;
        }
        self.service() == qualifier
            || match qualifier {
                "ddb" => self == Self::Table,
                "ec2" => matches!(self, Self::Instance | Self::Vpc),
                "cfn" => self == Self::Stack,
                "cw" | "cloudwatch" => matches!(self, Self::Alarm | Self::LogGroup),
                "apig" => self == Self::Api,
                "elasticfilesystem" => self == Self::Efs,
                "ecr" => matches!(self, Self::Repository | Self::PublicRepository),
                _ => false,
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub kind: ResourceKind,
    pub id: String,
    pub name: String,
    pub arn: String,
    pub region: String,
}

impl Resource {
    pub(super) fn new(
        kind: ResourceKind,
        id: impl Into<String>,
        name: impl Into<String>,
        region: &str,
    ) -> Self {
        Self {
            kind,
            id: id.into(),
            name: name.into(),
            arn: String::new(),
            region: region.into(),
        }
    }

    pub fn from_index(resource_type: &str, arn: &str, region: &str) -> Option<Self> {
        let kind = ResourceKind::ALL
            .into_iter()
            .find(|kind| kind.resource_types().split(',').any(|t| t == resource_type))?;
        let tail = arn.splitn(6, ':').nth(5)?;
        let id = match kind {
            ResourceKind::Bucket | ResourceKind::Queue => tail,
            ResourceKind::Database | ResourceKind::Function | ResourceKind::Alarm => {
                tail.split_once(':')?.1
            }
            ResourceKind::LogGroup => tail.strip_prefix("log-group:")?.trim_end_matches(":*"),
            ResourceKind::Api => tail.trim_start_matches('/').split('/').nth(1)?,
            ResourceKind::Stack => tail.strip_prefix("stack/")?.split('/').next()?,
            ResourceKind::User | ResourceKind::Role | ResourceKind::Group => {
                tail.rsplit('/').next()?
            }
            _ => tail.split_once('/')?.1,
        };
        if id.is_empty() {
            return None;
        }
        // Lambda versions/aliases and nested resources aren't separate finder targets.
        if kind == ResourceKind::Function && id.contains(':') {
            return None;
        }
        Some(Self {
            kind,
            id: id.into(),
            name: id.into(),
            arn: arn.into(),
            region: region.into(),
        })
    }

    /// Convert an explicit ARN to a finder target. Qualified functions, table
    /// streams and log streams open their parent view; the original ARN is retained.
    /// Types without a view remain visible as generic references.
    pub fn from_arn(arn: &str) -> Option<Self> {
        let mut parts = arn.splitn(6, ':');
        if parts.next()? != "arn" {
            return None;
        }
        let partition = parts.next()?;
        let service = parts.next()?;
        let region = parts.next()?;
        let account = parts.next()?;
        let tail = parts.next()?;
        if !matches!(
            partition,
            "aws" | "aws-cn" | "aws-us-gov" | "aws-iso" | "aws-iso-b" | "aws-iso-e" | "aws-iso-f"
        ) || service.is_empty()
            || tail.is_empty()
            || [partition, service, region, account]
                .iter()
                .any(|part| part.chars().any(char::is_whitespace))
        {
            return None;
        }
        let kind = match service {
            "s3" if !tail.contains('/') && !tail.contains(':') => Some(ResourceKind::Bucket),
            "s3" => None,
            "sqs" => Some(ResourceKind::Queue),
            _ => ResourceKind::ALL.into_iter().find(|kind| {
                kind.resource_types().split(',').any(|resource_type| {
                    let Some((expected_service, resource_type)) = resource_type.split_once(':')
                    else {
                        return false;
                    };
                    let tail = tail.trim_start_matches('/');
                    expected_service == service
                        && tail.strip_prefix(resource_type).is_some_and(|suffix| {
                            suffix.starts_with('/') || suffix.starts_with(':')
                        })
                })
            }),
        };
        let mut row = if let Some(kind) = kind {
            // Use the existing parser on a normalized parent ARN, then restore the
            // original reference for display, copying and account-scope checks.
            let parent_tail = match kind {
                ResourceKind::Function => tail
                    .rsplit_once(':')
                    .filter(|_| tail.matches(':').count() > 1)
                    .map_or(tail, |(parent, _)| parent),
                ResourceKind::Table => tail
                    .split("/stream/")
                    .next()
                    .unwrap_or(tail)
                    .split("/index/")
                    .next()
                    .unwrap_or(tail),
                ResourceKind::LogGroup => tail.split(":log-stream:").next().unwrap_or(tail),
                _ => tail,
            };
            let normalized = format!("arn:{partition}:{service}:{region}:{account}:{parent_tail}");
            Self::from_index(
                kind.resource_types().split(',').find(|resource_type| {
                    kind != ResourceKind::Api
                        || parent_tail.contains(resource_type.split_once(':').unwrap().1)
                })?,
                &normalized,
                region,
            )?
        } else {
            Self::new(ResourceKind::Reference, tail, tail, region)
        };
        row.arn = arn.into();
        Some(row)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub kinds: Vec<ResourceKind>,
}

impl SearchQuery {
    pub fn parse(input: &str) -> Result<Self> {
        let mut qualifiers = Vec::new();
        let mut words = Vec::new();
        for word in input.split_whitespace() {
            if let Some(service) = word.strip_prefix('@') {
                let service = service.to_ascii_lowercase();
                if !ResourceKind::ALL
                    .iter()
                    .any(|k| k.accepts_qualifier(&service))
                {
                    bail!(
                        "Unknown service @{service}; try @s3, @dynamodb, @ec2, @rds, @iam, @lambda"
                    );
                }
                qualifiers.push(service);
            } else {
                words.push(word);
            }
        }
        let text = words.join(" ").to_lowercase();
        let kinds = ResourceKind::ALL
            .into_iter()
            .filter(|kind| {
                qualifiers.is_empty() || qualifiers.iter().any(|q| kind.accepts_qualifier(q))
            })
            .collect();
        Ok(Self { text, kinds })
    }
    pub fn ready(&self) -> bool {
        self.text.chars().filter(|c| !c.is_whitespace()).count() >= 3
    }
    /// Fast name/ID/ARN matching for the fallback inventory cache.
    pub fn matches(&self, resource: &Resource) -> bool {
        if resource.kind == ResourceKind::Reference {
            let service = resource.arn.split(':').nth(2).unwrap_or_default();
            let service_matches = self.kinds.len() == ResourceKind::ALL.len()
                || self.kinds.iter().any(|kind| kind.service() == service);
            return service_matches
                && [&resource.id, &resource.name, &resource.arn]
                    .iter()
                    .any(|value| value.to_lowercase().contains(&self.text));
        }
        self.kinds.contains(&resource.kind)
            && [&resource.id, &resource.name, &resource.arn]
                .iter()
                .any(|value| value.to_lowercase().contains(&self.text))
    }
    fn index_query(&self, region: &str) -> String {
        // Resource Explorer requires bare terms for prefix matching: `fsx*`,
        // not `"fsx"*` (the latter changes the query's meaning). Only ordinary
        // resource-name tokens may be bare; quote syntax-bearing literals so
        // user input cannot inject filters, negation, or wildcards.
        let terms = self
            .text
            .split_whitespace()
            .map(|term| {
                if !term.starts_with('-')
                    && term
                        .chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
                {
                    format!("{term}*")
                } else {
                    let escaped = term.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("\"{escaped}\"")
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let types = self
            .kinds
            .iter()
            .map(|k| k.resource_types())
            .collect::<Vec<_>>()
            .join(",");
        format!("{terms} resourcetype:{types} region:{region},global")
    }
}

#[derive(Debug)]
pub enum SearchUpdate {
    Provider(&'static str),
    Results(Vec<Resource>),
    Warning(String),
    Finished { limited: bool },
}

#[derive(Default)]
pub struct SearchCache {
    inventory: HashMap<ResourceKind, (Instant, Vec<Resource>, bool)>,
    index_unavailable: Option<Instant>,
}
pub type SharedSearchCache = Arc<Mutex<SearchCache>>;

/// The caller owns cancellation by aborting this future. All fallback futures
/// are polled here (not detached tasks), so cancellation drops outstanding reads.
pub async fn search(
    config: AwsConfig,
    query: SearchQuery,
    cache: SharedSearchCache,
    tx: mpsc::Sender<SearchUpdate>,
) {
    if !query.ready() {
        return;
    }
    let try_index = cache
        .lock()
        .await
        .index_unavailable
        .is_none_or(|at| at.elapsed() >= CACHE_TTL);
    if try_index {
        let _ = tx
            .send(SearchUpdate::Provider("Indexed prefix search"))
            .await;
        match tokio::time::timeout(Duration::from_secs(12), search_index(&config, &query, &tx))
            .await
        {
            Ok(Ok(limited)) => {
                let _ = tx.send(SearchUpdate::Finished { limited }).await;
                return;
            }
            _ => {
                cache.lock().await.index_unavailable = Some(Instant::now());
                let _ = tx
                    .send(SearchUpdate::Warning(
                        "Indexed search unavailable; using supported-service inventories".into(),
                    ))
                    .await;
            }
        }
    }
    let _ = tx
        .send(SearchUpdate::Provider(
            "Parallel inventory search (60s cache)",
        ))
        .await;
    let mut work = stream::iter(query.kinds.clone())
        .map(|kind| {
            let config = config.clone();
            let cache = cache.clone();
            async move {
                if let Some((at, rows, limited)) = cache.lock().await.inventory.get(&kind) {
                    if at.elapsed() < CACHE_TTL {
                        return (kind, Ok((rows.clone(), *limited)));
                    }
                }
                let result = match tokio::time::timeout(
                    Duration::from_secs(20),
                    inventory::fetch(&config, kind),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(anyhow::anyhow!("request timed out")),
                };
                if let Ok((rows, limited)) = &result {
                    cache
                        .lock()
                        .await
                        .inventory
                        .insert(kind, (Instant::now(), rows.clone(), *limited));
                }
                (kind, result)
            }
        })
        .buffer_unordered(4);
    let mut limited = false;
    let mut total = 0;
    while let Some((kind, result)) = work.next().await {
        match result {
            Ok((rows, capped)) => {
                limited |= capped;
                let matches: Vec<_> = rows.into_iter().filter(|r| query.matches(r)).collect();
                let remaining = RESULT_LIMIT.saturating_sub(total);
                limited |= matches.len() > remaining;
                let matches: Vec<_> = matches.into_iter().take(remaining).collect();
                total += matches.len();
                if tx.send(SearchUpdate::Results(matches)).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                if tx
                    .send(SearchUpdate::Warning(format!(
                        "{} {}: {error}",
                        kind.service(),
                        kind.label()
                    )))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }
    }
    let _ = tx.send(SearchUpdate::Finished { limited }).await;
}

async fn search_index(
    config: &AwsConfig,
    query: &SearchQuery,
    tx: &mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.resource_explorer_client();
    let mut token = None;
    let mut count = 0;
    let mut limited = false;
    loop {
        let page = client
            .search()
            .query_string(query.index_query(&config.region))
            .max_results(100)
            .set_next_token(token.clone())
            .send()
            .await?;
        if page.count().is_some_and(|c| c.complete() != Some(true)) {
            limited = true;
        }
        let rows: Vec<_> = page
            .resources()
            .iter()
            .filter(|r| r.owning_account_id() == Some(config.account_id.as_str()))
            .filter_map(|r| Resource::from_index(r.resource_type()?, r.arn()?, r.region()?))
            .filter(|r| query.kinds.contains(&r.kind))
            .collect();
        count += rows.len();
        tx.send(SearchUpdate::Results(rows))
            .await
            .map_err(|_| anyhow::anyhow!("search cancelled"))?;
        let next = page.next_token().map(str::to_owned);
        if next.is_none() {
            break;
        }
        if next == token || count >= RESULT_LIMIT {
            limited = true;
            break;
        }
        token = next;
    }
    Ok(limited)
}

#[cfg(test)]
mod tests;
