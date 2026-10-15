//! Read-only SNS Topics inventory. ListTopics supplies all three columns;
//! no per-topic GetTopicAttributes requests are needed.
use crate::config::AwsConfig;
use anyhow::{bail, Context, Result};
use std::{collections::HashSet, time::Duration};

const LIST_TIMEOUT: Duration = Duration::from_secs(30);
// Fail explicitly at the bound rather than presenting an incomplete inventory.
const MAX_PAGES: usize = 1000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SnsTopic {
    pub name: String,
    pub arn: String,
    pub topic_type: String,
}
impl SnsTopic {
    pub fn from_arn(arn: &str) -> Result<Self> {
        let parts: Vec<_> = arn.splitn(6, ':').collect();
        if parts.first() != Some(&"arn") || parts.get(2) != Some(&"sns") {
            bail!("not an SNS topic ARN: {arn:?}");
        }
        if parts.len() != 6
            || parts[1].is_empty()
            || parts[3].is_empty()
            || parts[4].len() != 12
            || !parts[4].bytes().all(|c| c.is_ascii_digit())
        {
            bail!("malformed SNS topic ARN: {arn:?}");
        }
        let name = parts[5];
        let fifo = name.ends_with(".fifo");
        let base_name = name.strip_suffix(".fifo").unwrap_or(name);
        if base_name.is_empty()
            || name.len() > 256
            || !base_name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        {
            bail!("malformed SNS topic ARN (invalid topic name): {arn:?}");
        }
        Ok(Self {
            name: name.into(),
            arn: arn.into(),
            topic_type: if fifo { "FIFO" } else { "Standard" }.into(),
        })
    }
}

pub struct SnsClient {
    config: AwsConfig,
}
impl SnsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }
    pub async fn list_topics(&self) -> Result<Vec<SnsTopic>> {
        tokio::time::timeout(LIST_TIMEOUT, self.list_topics_inner(MAX_PAGES))
            .await
            .map_err(|_| anyhow::anyhow!("SNS ListTopics timed out after {LIST_TIMEOUT:?}"))?
    }
    async fn list_topics_inner(&self, max_pages: usize) -> Result<Vec<SnsTopic>> {
        let client = self.config.sns_client();
        let mut topics = Vec::new();
        let mut token = None;
        let mut seen = HashSet::new();
        for _ in 0..max_pages {
            // ListTopics has a fixed page size; there is no maxResults parameter.
            let page = client
                .list_topics()
                .set_next_token(token)
                .send()
                .await
                .context("SNS ListTopics request failed")?;
            for topic in page.topics() {
                let arn = topic
                    .topic_arn()
                    .filter(|arn| !arn.is_empty())
                    .context("SNS ListTopics returned a topic with no ARN")?;
                topics.push(SnsTopic::from_arn(arn)?);
            }
            let next = page.next_token().filter(|t| !t.is_empty());
            let Some(next) = next else {
                topics.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
                return Ok(topics);
            };
            if !seen.insert(next.to_owned()) {
                bail!("SNS ListTopics returned a repeated pagination token; aborting");
            }
            token = Some(next.to_owned());
        }
        bail!("SNS ListTopics exceeded {max_pages} pages; inventory is incomplete")
    }
}
#[cfg(test)]
mod tests;
