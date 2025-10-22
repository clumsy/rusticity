use anyhow::Result;
use aws_sdk_cloudwatchlogs::Client;
use chrono::DateTime;

use crate::config::AwsConfig;
use crate::types::{LogEvent, LogGroup, LogStream};

pub struct CloudWatchClient {
    client: Client,
    config: AwsConfig,
}

impl CloudWatchClient {
    pub async fn new(config: AwsConfig) -> Result<Self> {
        let client = config.cloudwatch_logs_client().await;
        Ok(Self { client, config })
    }

    pub fn config(&self) -> &AwsConfig {
        &self.config
    }

    pub fn dummy(config: AwsConfig) -> Self {
        let aws_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .build();
        let client = Client::new(&aws_config);
        Self { client, config }
    }

    pub async fn list_log_groups(&self) -> Result<Vec<LogGroup>> {
        let resp = self.client.describe_log_groups().send().await?;

        let groups = resp
            .log_groups()
            .iter()
            .map(|g| LogGroup {
                name: g.log_group_name().unwrap_or("").to_string(),
                creation_time: g
                    .creation_time()
                    .map(|t| DateTime::from_timestamp_millis(t).unwrap_or_default()),
                stored_bytes: g.stored_bytes(),
                retention_days: g.retention_in_days(),
                log_class: g.log_group_class().map(|c| c.as_str().to_string()),
                arn: g.log_group_arn().map(|a| a.to_string()),
            })
            .collect();

        Ok(groups)
    }

    pub async fn list_log_streams(&self, log_group: &str) -> Result<Vec<LogStream>> {
        let mut streams = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .describe_log_streams()
                .log_group_name(log_group)
                .order_by(aws_sdk_cloudwatchlogs::types::OrderBy::LastEventTime)
                .descending(true);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let resp = request.send().await?;

            streams.extend(resp.log_streams().iter().map(|s| {
                LogStream {
                    name: s.log_stream_name().unwrap_or("").to_string(),
                    creation_time: s
                        .creation_time()
                        .map(|t| DateTime::from_timestamp_millis(t).unwrap_or_default()),
                    last_event_time: s
                        .last_event_timestamp()
                        .map(|t| DateTime::from_timestamp_millis(t).unwrap_or_default()),
                }
            }));

            if streams.len() >= 100 {
                break;
            }

            next_token = resp.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(streams)
    }

    pub async fn get_log_events(
        &self,
        log_group: &str,
        log_stream: &str,
        backward_token: Option<String>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<(Vec<LogEvent>, bool, Option<String>)> {
        let prev_token = backward_token.clone();
        let mut request = self
            .client
            .get_log_events()
            .log_group_name(log_group)
            .log_stream_name(log_stream)
            .set_start_from_head(Some(false))
            .set_limit(Some(25));

        if let Some(start) = start_time {
            request = request.set_start_time(Some(start));
        }

        if let Some(end) = end_time {
            request = request.set_end_time(Some(end));
        }

        if let Some(token) = backward_token {
            request = request.set_next_token(Some(token));
        }

        let resp = request.send().await?;

        let mut events: Vec<LogEvent> = resp
            .events()
            .iter()
            .map(|e| LogEvent {
                timestamp: DateTime::from_timestamp_millis(e.timestamp().unwrap_or(0))
                    .unwrap_or_default(),
                message: e.message().unwrap_or("").to_string(),
            })
            .collect();

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let next_backward_token = resp.next_backward_token().map(|s| s.to_string());
        let has_more = next_backward_token.is_some()
            && next_backward_token != prev_token
            && !events.is_empty();

        Ok((events, has_more, next_backward_token))
    }

    pub async fn start_query(
        &self,
        log_group_names: Vec<String>,
        query_string: String,
        start_time: i64,
        end_time: i64,
    ) -> Result<String> {
        let resp = self
            .client
            .start_query()
            .set_log_group_names(Some(log_group_names))
            .query_string(query_string)
            .start_time(start_time / 1000)
            .end_time(end_time / 1000)
            .send()
            .await?;

        Ok(resp.query_id().unwrap_or("").to_string())
    }

    pub async fn get_query_results(
        &self,
        query_id: &str,
    ) -> Result<(String, Vec<Vec<(String, String)>>)> {
        let resp = self
            .client
            .get_query_results()
            .query_id(query_id)
            .send()
            .await?;

        let status = resp
            .status()
            .map(|s| s.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let results: Vec<Vec<(String, String)>> = resp
            .results()
            .iter()
            .map(|result_row| {
                result_row
                    .iter()
                    .map(|field| {
                        (
                            field.field().unwrap_or("").to_string(),
                            field.value().unwrap_or("").to_string(),
                        )
                    })
                    .collect()
            })
            .collect();

        Ok((status, results))
    }
}
