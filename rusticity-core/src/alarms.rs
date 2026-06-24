use crate::config::AwsConfig;
use anyhow::Result;

pub struct AlarmsClient {
    config: AwsConfig,
}

impl AlarmsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_alarms(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            u32,
            String,
            f64,
            bool,
            String,
            String,
            String,
            String,
            String,
            String,
        )>,
    > {
        let client = self.config.cloudwatch_client();

        let resp = client.describe_alarms().send().await?;

        let mut alarms = Vec::new();

        for alarm in resp.metric_alarms() {
            let name = alarm.alarm_name().unwrap_or("").to_string();
            let state = alarm
                .state_value()
                .map(|s| s.as_str())
                .unwrap_or("INSUFFICIENT_DATA")
                .to_string();
            let state_updated = alarm
                .state_updated_timestamp()
                .map(|t| {
                    let dt = chrono::DateTime::parse_from_rfc3339(&t.to_string())
                        .unwrap_or_else(|_| chrono::Utc::now().into());
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();
            let description = alarm.alarm_description().unwrap_or("").to_string();
            let metric_name = alarm.metric_name().unwrap_or("").to_string();
            let namespace = alarm.namespace().unwrap_or("").to_string();
            let statistic = alarm
                .statistic()
                .map(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let period = alarm.period().unwrap_or(0) as u32;
            let comparison = alarm
                .comparison_operator()
                .map(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let threshold = alarm.threshold().unwrap_or(0.0);
            let actions_enabled = alarm.actions_enabled().unwrap_or(false);
            let state_reason = alarm.state_reason().unwrap_or("").to_string();

            let resource = alarm
                .dimensions()
                .iter()
                .map(|d| format!("{}={}", d.name().unwrap_or(""), d.value().unwrap_or("")))
                .collect::<Vec<_>>()
                .join(", ");

            let dimensions = resource.clone();
            let expression = if !alarm.metrics().is_empty() {
                "Expression".to_string()
            } else {
                String::new()
            };
            let alarm_type = if !alarm.metrics().is_empty() {
                "Metric math"
            } else {
                "Metric"
            }
            .to_string();
            let cross_account = "".to_string();

            alarms.push((
                name,
                state,
                state_updated,
                description,
                metric_name,
                namespace,
                statistic,
                period,
                comparison,
                threshold,
                actions_enabled,
                state_reason,
                resource,
                dimensions,
                expression,
                alarm_type,
                cross_account,
            ));
        }

        Ok(alarms)
    }

    pub async fn get_metric_statistics(
        &self,
        namespace: &str,
        metric_name: &str,
        statistic: &str,
        period: i32,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client();

        let resp = client
            .get_metric_statistics()
            .namespace(namespace)
            .metric_name(metric_name)
            .statistics(aws_sdk_cloudwatch::types::Statistic::from(statistic))
            .period(period)
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                end_time.timestamp_millis(),
            ))
            .send()
            .await?;

        let mut data: Vec<(i64, f64)> = resp
            .datapoints()
            .iter()
            .filter_map(|dp| {
                let timestamp = dp.timestamp()?.as_secs_f64() as i64;
                let value = match statistic {
                    "Average" => dp.average(),
                    "Sum" => dp.sum(),
                    "Minimum" => dp.minimum(),
                    "Maximum" => dp.maximum(),
                    "SampleCount" => dp.sample_count(),
                    _ => dp.average(),
                }?;
                Some((timestamp, value))
            })
            .collect();

        data.sort_by_key(|(ts, _)| *ts);
        Ok(data)
    }
}
