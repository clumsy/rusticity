//! Resource Explorer › Resources: one Search with the default view of the
//! Region, as the console Resource search page runs it. Read-only.
//!
//! The query string is passed as typed; Resource Explorer parses keywords and
//! filters (`service:`, `region:`, `tag:` and the rest). Search returns at
//! most 1,000 resources; `complete` says whether the view holds more.
use crate::config::AwsConfig;
use anyhow::Result;
use aws_sdk_resourceexplorer2::types::Resource as SdkResource;
use aws_smithy_types::Document;

/// The most resources one Search returns.
pub const RESULT_LIMIT: usize = 1000;
const PAGE_SIZE: i32 = 1000;

/// One resource found, as the console results list it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExplorerResource {
    pub arn: String,
    /// The resource ID from the ARN, such as `i-0123456789abcdef0`.
    pub identifier: String,
    /// Such as `ec2`.
    pub service: String,
    /// Such as `ec2:instance`.
    pub resource_type: String,
    /// A Region code, or `global`.
    pub region: String,
    pub owner_account: String,
    /// The name of the application in the `awsApplication` tag.
    pub application: String,
    /// `None` when the view does not include tags; sorted by key.
    pub tags: Option<Vec<(String, String)>>,
    pub last_reported_at: String,
}

/// The results of one search.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceSearch {
    pub resources: Vec<ExplorerResource>,
    /// False when the view holds more matches than were returned.
    pub complete: bool,
    /// The view that answered, when the service names it.
    pub view_arn: String,
}

/// `i-0abc` from `arn:aws:ec2:r:a:instance/i-0abc`, `orders` from
/// `arn:aws:lambda:r:a:function:orders`, the name of an S3 bucket.
pub fn identifier_from_arn(arn: &str) -> String {
    let resource = arn
        .splitn(6, ':')
        .nth(5)
        .unwrap_or_default()
        .trim_start_matches('/');
    match resource.find(['/', ':']) {
        Some(at) => resource[at + 1..].to_string(),
        None => resource.to_string(),
    }
}

/// `MyApp` from `arn:aws:resource-groups:r:a:group/MyApp/0abc`; any other
/// value as it is.
fn application_name(value: &str) -> String {
    value
        .split_once(":group/")
        .and_then(|(_, rest)| rest.split('/').next())
        .filter(|name| !name.is_empty())
        .unwrap_or(value)
        .to_string()
}

fn text(document: &Document) -> Option<&str> {
    match document {
        Document::String(s) => Some(s),
        _ => None,
    }
}

/// The `tags` property: a list of `{"Key": .., "Value": ..}`.
fn tags_from(data: &Document) -> Vec<(String, String)> {
    let Document::Array(items) = data else {
        return Vec::new();
    };
    let mut tags: Vec<(String, String)> = items
        .iter()
        .filter_map(|item| {
            let Document::Object(map) = item else {
                return None;
            };
            let key = map.get("Key").and_then(text)?;
            let value = map.get("Value").and_then(text).unwrap_or_default();
            Some((key.to_string(), value.to_string()))
        })
        .collect();
    tags.sort();
    tags
}

impl ExplorerResource {
    fn from_sdk(resource: &SdkResource) -> Option<Self> {
        let arn = resource.arn().filter(|arn| !arn.is_empty())?;
        let tags = resource
            .properties()
            .iter()
            .find(|p| p.name() == Some("tags"))
            .map(|p| p.data().map(tags_from).unwrap_or_default());
        let application = tags
            .iter()
            .flatten()
            .find(|(key, _)| key == "awsApplication")
            .map(|(_, value)| application_name(value))
            .unwrap_or_default();
        Some(Self {
            arn: arn.to_string(),
            identifier: identifier_from_arn(arn),
            service: resource.service().unwrap_or_default().into(),
            resource_type: resource.resource_type().unwrap_or_default().into(),
            region: resource.region().unwrap_or_default().into(),
            owner_account: resource.owning_account_id().unwrap_or_default().into(),
            application,
            tags,
            last_reported_at: resource
                .last_reported_at()
                .and_then(|t| t.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
                .unwrap_or_default(),
        })
    }
}

/// Search the default view of the Region with `query` (empty: everything
/// the view holds, up to the limit).
pub async fn search(config: &AwsConfig, query: &str) -> Result<ResourceSearch> {
    let client = config.resource_explorer_client();
    let mut found = ResourceSearch {
        complete: true,
        ..Default::default()
    };
    let mut token: Option<String> = None;
    loop {
        let page = client
            .search()
            .query_string(query)
            .max_results(PAGE_SIZE)
            .set_next_token(token.clone())
            .send()
            .await?;
        if let Some(view) = page.view_arn() {
            found.view_arn = view.to_string();
        }
        if page.count().is_some_and(|c| c.complete() == Some(false)) {
            found.complete = false;
        }
        found.resources.extend(
            page.resources()
                .iter()
                .filter_map(ExplorerResource::from_sdk),
        );
        let next = page
            .next_token()
            .filter(|t| !t.is_empty())
            .map(str::to_owned);
        match next {
            None => break,
            Some(next)
                if found.resources.len() >= RESULT_LIMIT || token.as_ref() == Some(&next) =>
            {
                found.complete = false;
                break;
            }
            Some(next) => token = Some(next),
        }
    }
    if found.resources.len() > RESULT_LIMIT {
        found.resources.truncate(RESULT_LIMIT);
        found.complete = false;
    }
    Ok(found)
}

#[cfg(test)]
mod tests;
