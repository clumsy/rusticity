use crate::config::AwsConfig;
use anyhow::Result;

pub struct CloudTrailClient {
    config: AwsConfig,
}

impl CloudTrailClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn lookup_events(
        &self,
        max_results: Option<i32>,
        next_token: Option<String>,
    ) -> Result<(
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        )>,
        Option<String>,
    )> {
        let client = self.config.cloudtrail_client().await;

        let mut request = client.lookup_events();
        if let Some(max) = max_results {
            request = request.max_results(max);
        }
        if let Some(token) = next_token {
            request = request.next_token(token);
        }

        let resp = request.send().await?;

        let mut events = Vec::new();
        for event in resp.events() {
            let event_name = event.event_name().unwrap_or("").to_string();
            let event_time = event
                .event_time()
                .map(|t| {
                    let dt = chrono::DateTime::parse_from_rfc3339(&t.to_string())
                        .unwrap_or_else(|_| chrono::Utc::now().into());
                    format!("{} (UTC)", dt.format("%Y-%m-%d %H:%M:%S"))
                })
                .unwrap_or_default();
            let username = event.username().unwrap_or("").to_string();
            let event_source = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["eventSource"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_default();
            let resource_type = event
                .resources()
                .first()
                .and_then(|r| r.resource_type())
                .unwrap_or("")
                .to_string();
            let resource_name = event
                .resources()
                .first()
                .and_then(|r| r.resource_name())
                .unwrap_or("")
                .to_string();
            let read_only = event.read_only().map(|b| b.to_string()).unwrap_or_default();
            let aws_region = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["awsRegion"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_default();
            let event_id = event.event_id().unwrap_or("").to_string();
            let access_key_id = event.access_key_id().unwrap_or("").to_string();
            let source_ip = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["sourceIPAddress"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_default();
            let error_code = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["errorCode"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_default();
            let request_id = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["requestID"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_default();
            let event_type = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["eventType"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_default();
            let cloud_trail_event_json = event
                .cloud_trail_event()
                .and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                })
                .unwrap_or_else(|| "{}".to_string());

            events.push((
                event_name,
                event_time,
                username,
                event_source,
                resource_type,
                resource_name,
                read_only,
                aws_region,
                event_id,
                access_key_id,
                source_ip,
                error_code,
                request_id,
                event_type,
                cloud_trail_event_json,
            ));
        }

        Ok((events, resp.next_token().map(|s| s.to_string())))
    }

    /// Fetch minimal data (1 event) just to get the next token
    pub async fn get_next_token(&self, current_token: String) -> Result<Option<String>> {
        let client = self.config.cloudtrail_client().await;
        let resp = client
            .lookup_events()
            .max_results(1)
            .next_token(current_token)
            .send()
            .await?;
        Ok(resp.next_token().map(|s| s.to_string()))
    }
}
