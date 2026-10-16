//! Read-only SNS subscriptions for a single topic. ListSubscriptionsByTopic
//! supplies every column directly — no per-subscription GetSubscriptionAttributes
//! requests, so there is no N+1 fan-out. Subscription status is read from the
//! only authoritative source the list call exposes: the `SubscriptionArn`
//! literal (`PendingConfirmation` / `Deleted`) versus a real subscription ARN.
use super::{SnsClient, SnsTopic};
use crate::pagination::{Pagination, StopReason};
use anyhow::{anyhow, bail, Context, Result};
use std::time::Duration;

// Fail explicitly at the bound rather than presenting an incomplete list.
const LIST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PAGES: usize = 1000;

// SNS returns these literals in the SubscriptionArn field in place of a real
// ARN; they are the only in-band signal of a subscription's lifecycle state.
const PENDING_ARN: &str = "PendingConfirmation";
const DELETED_ARN: &str = "Deleted";

// Human-facing status strings. Endpoint/protocol/owner presentation (blank vs
// "-") is the UI's responsibility; we leave the raw values here.
const STATUS_PENDING: &str = "Pending confirmation";
const STATUS_DELETED: &str = "Deleted";
const STATUS_CONFIRMED: &str = "Confirmed";
const STATUS_UNKNOWN: &str = "Unknown";

/// One subscription to an SNS topic, as reported by ListSubscriptionsByTopic.
///
/// `status` is derived, not asserted: a subscription is only `Confirmed` when
/// its `subscription_arn` is a genuine ARN scoped to the requested topic. An
/// unrecognised value maps to `Unknown` rather than being presented as
/// confirmed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SnsSubscription {
    pub subscription_arn: String,
    pub topic_arn: String,
    pub protocol: String,
    pub endpoint: String,
    pub owner: String,
    pub status: String,
}

impl SnsSubscription {
    /// Classify a subscription's status from its ARN literal, scoped to the
    /// topic that was requested. `PendingConfirmation` and `Deleted` are the
    /// documented literals; a real ARN that is prefixed by the topic ARN plus a
    /// `:` suffix is `Confirmed`; anything else is `Unknown` (never silently
    /// reported as confirmed).
    fn classify_status(subscription_arn: &str, topic_arn: &str) -> &'static str {
        match subscription_arn {
            "" => STATUS_UNKNOWN,
            PENDING_ARN => STATUS_PENDING,
            DELETED_ARN => STATUS_DELETED,
            arn => {
                let confirmed = arn
                    .strip_prefix(topic_arn)
                    .is_some_and(|suffix| suffix.starts_with(':') && suffix.len() > 1);
                if confirmed {
                    STATUS_CONFIRMED
                } else {
                    STATUS_UNKNOWN
                }
            }
        }
    }

    fn from_sdk(sub: &aws_sdk_sns::types::Subscription, requested_topic_arn: &str) -> Self {
        let subscription_arn = sub.subscription_arn().unwrap_or_default().to_string();
        // Trust the response topic ARN only if present; fall back to the
        // requested ARN so a confirmed subscription always carries its topic.
        let topic_arn = sub
            .topic_arn()
            .filter(|arn| !arn.is_empty())
            .unwrap_or(requested_topic_arn)
            .to_string();
        let status = Self::classify_status(&subscription_arn, requested_topic_arn).to_string();
        Self {
            subscription_arn,
            topic_arn,
            protocol: sub.protocol().unwrap_or_default().to_string(),
            endpoint: sub.endpoint().unwrap_or_default().to_string(),
            owner: sub.owner().unwrap_or_default().to_string(),
            status,
        }
    }
}

impl SnsClient {
    /// List every subscription to `topic_arn` with a single bounded, timed read.
    ///
    /// The ARN is validated and checked against the active config's account and
    /// Region before any request. The read is complete-or-error: a denied or
    /// failing call surfaces the underlying AWS error (with its metadata chain)
    /// rather than an empty success. Results are deterministically sorted by
    /// protocol, then endpoint, then subscription ARN; duplicate pending
    /// subscriptions sharing an endpoint are retained, never silently dropped.
    pub async fn list_subscriptions_by_topic(
        &self,
        topic_arn: &str,
    ) -> Result<Vec<SnsSubscription>> {
        self.validate_topic_target(topic_arn)?;
        tokio::time::timeout(
            LIST_TIMEOUT,
            self.list_subscriptions_by_topic_inner(topic_arn, MAX_PAGES),
        )
        .await
        .map_err(|_| anyhow!("SNS ListSubscriptionsByTopic timed out after {LIST_TIMEOUT:?}"))?
    }

    /// Reject a missing/malformed topic ARN and known account/Region mismatches
    /// against the active config before spending a request.
    fn validate_topic_target(&self, topic_arn: &str) -> Result<()> {
        if topic_arn.is_empty() {
            bail!("SNS topic ARN is required");
        }
        // Reuses the module's own ARN validation (service, account, name rules).
        SnsTopic::from_arn(topic_arn).context("invalid SNS topic ARN")?;
        let parts: Vec<_> = topic_arn.splitn(6, ':').collect();
        let (region, account) = (parts[3], parts[4]);
        if !self.config.region.is_empty() && region != self.config.region {
            bail!(
                "SNS topic ARN Region {region:?} does not match the active Region {:?}",
                self.config.region
            );
        }
        if !self.config.account_id.is_empty() && account != self.config.account_id {
            bail!(
                "SNS topic ARN account {account:?} does not match the active account {:?}",
                self.config.account_id
            );
        }
        Ok(())
    }

    async fn list_subscriptions_by_topic_inner(
        &self,
        topic_arn: &str,
        max_pages: usize,
    ) -> Result<Vec<SnsSubscription>> {
        let client = self.config.sns_client();
        let mut subscriptions = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(max_pages);
        for _ in 0..max_pages {
            // ListSubscriptionsByTopic takes only TopicArn and NextToken; there
            // is no maxResults parameter on the builder.
            let page = client
                .list_subscriptions_by_topic()
                .topic_arn(topic_arn)
                .set_next_token(token)
                .send()
                .await
                .context("SNS ListSubscriptionsByTopic request failed")?;

            // If the service echoes a TopicArn, a mismatch is a hard error; when
            // it omits one we scope results to the requested ARN explicitly.
            for sub in page.subscriptions() {
                if let Some(returned) = sub.topic_arn().filter(|arn| !arn.is_empty()) {
                    if returned != topic_arn {
                        bail!(
                            "SNS ListSubscriptionsByTopic returned a subscription for \
                             topic {returned:?}, expected {topic_arn:?}"
                        );
                    }
                }
                subscriptions.push(SnsSubscription::from_sdk(sub, topic_arn));
            }

            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => {
                    sort_subscriptions(&mut subscriptions);
                    return Ok(subscriptions);
                }
                Err(StopReason::RepeatedToken) => {
                    bail!(
                        "SNS ListSubscriptionsByTopic returned a repeated pagination token; \
                         aborting"
                    );
                }
                Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
            }
        }
        bail!(
            "SNS ListSubscriptionsByTopic exceeded {max_pages} pages; subscription list is \
             incomplete"
        )
    }
}

/// Deterministic order: protocol, then endpoint, then subscription ARN. Stable
/// so duplicate pending subscriptions (same endpoint, literal `PendingConfirmation`
/// ARN) keep their relative order rather than being deduplicated away.
fn sort_subscriptions(subscriptions: &mut [SnsSubscription]) {
    subscriptions.sort_by(|a, b| {
        a.protocol
            .cmp(&b.protocol)
            .then_with(|| a.endpoint.cmp(&b.endpoint))
            .then_with(|| a.subscription_arn.cmp(&b.subscription_arn))
    });
}

#[cfg(test)]
mod tests;
