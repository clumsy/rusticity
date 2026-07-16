use crate::config::AwsConfig;
use anyhow::Result;

pub struct AlarmsClient {
    config: AwsConfig,
}

/// A single entry in an alarm's metric math definition.
#[derive(Debug, Clone)]
pub struct SubMetric {
    pub id: String,
    pub label: String,
    pub expression: Option<String>,
    pub namespace: Option<String>,
    pub metric_name: Option<String>,
    pub dimensions: Vec<(String, String)>,
    pub period: i32,
    pub stat: String,
    pub return_data: bool,
}

/// All alarm fields returned by describe_alarms, ready to be used by the term layer.
#[derive(Debug, Clone)]
pub struct AlarmData {
    pub name: String,
    pub state: String,
    pub state_updated: String,
    pub description: String,
    pub metric_name: String,
    pub namespace: String,
    pub statistic: String,
    pub period: u32,
    pub comparison: String,
    pub threshold: f64,
    pub actions_enabled: bool,
    pub state_reason: String,
    pub resource: String,
    pub dimensions: String,
    pub expression: String,
    pub alarm_type: String,
    pub cross_account: String,
    pub alarm_arn: String,
    pub datapoints_to_alarm: u32,
    pub evaluation_periods: u32,
    pub treat_missing_data: String,
    pub evaluate_low_sample_percentile: String,
    /// Non-empty for metric math alarms; contains the sub-metrics and expression.
    pub sub_metrics: Vec<SubMetric>,
}

/// Parameters for a simple GetMetricStatistics call.
pub struct MetricStatsRequest<'a> {
    pub namespace: &'a str,
    pub metric_name: &'a str,
    pub statistic: &'a str,
    pub period: i32,
    pub dimensions: &'a [(String, String)],
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

impl AlarmsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_alarms(&self) -> Result<Vec<AlarmData>> {
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
            let alarm_arn = alarm.alarm_arn().unwrap_or("").to_string();
            let datapoints_to_alarm = alarm.datapoints_to_alarm().unwrap_or(0) as u32;
            let evaluation_periods = alarm.evaluation_periods().unwrap_or(0) as u32;
            let treat_missing_data = alarm
                .treat_missing_data()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let evaluate_low_sample_percentile = alarm
                .evaluate_low_sample_count_percentile()
                .unwrap_or("")
                .to_string();

            // Extract sub-metrics for metric math alarms
            let sub_metrics: Vec<SubMetric> = alarm
                .metrics()
                .iter()
                .map(|mq| {
                    let id = mq.id().unwrap_or("").to_string();
                    let label = mq.label().unwrap_or("").to_string();
                    let expr = mq.expression().map(|s| s.to_string());
                    let (ns, mn, dims, period, stat) = if let Some(ms) = mq.metric_stat() {
                        let ns = ms
                            .metric()
                            .and_then(|m| m.namespace())
                            .unwrap_or("")
                            .to_string();
                        let mn = ms
                            .metric()
                            .and_then(|m| m.metric_name())
                            .unwrap_or("")
                            .to_string();
                        let dims: Vec<(String, String)> = ms
                            .metric()
                            .map(|m| {
                                m.dimensions()
                                    .iter()
                                    .map(|d| {
                                        (
                                            d.name().unwrap_or("").to_string(),
                                            d.value().unwrap_or("").to_string(),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let period = ms.period().unwrap_or(60);
                        let stat = ms.stat().unwrap_or("Average").to_string();
                        (Some(ns), Some(mn), dims, period, stat)
                    } else {
                        // Expression entry — read period directly from MetricDataQuery if set
                        let period = mq.period().unwrap_or(60);
                        (None, None, vec![], period, String::new())
                    };
                    SubMetric {
                        id,
                        label,
                        expression: expr,
                        namespace: ns,
                        metric_name: mn,
                        dimensions: dims,
                        period,
                        stat,
                        return_data: mq.return_data().unwrap_or(false),
                    }
                })
                .collect();

            alarms.push(AlarmData {
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
                cross_account: String::new(),
                alarm_arn,
                datapoints_to_alarm,
                evaluation_periods,
                treat_missing_data,
                evaluate_low_sample_percentile,
                sub_metrics,
            });
        }

        Ok(alarms)
    }

    /// Fetch the time series for a metric math alarm by calling GetMetricData with the
    /// alarm's sub-metric queries. Returns (timestamps_ms, values) for the visible expression.
    pub async fn get_metric_math_data(
        &self,
        sub_metrics: &[SubMetric],
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<(i64, f64)>> {
        use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};

        let client = self.config.cloudwatch_client();

        // Find a representative period — prefer raw metric entries, fall back to expression entry period.
        let default_period: Option<i32> = sub_metrics
            .iter()
            .filter(|sm| sm.period > 0)
            .map(|sm| sm.period)
            .next();

        // Determine which entry should return data.
        // If no entry has return_data=true (e.g. single Metrics Insights alarm),
        // force return_data on the first expression entry.
        let visible_id: Option<String> = sub_metrics
            .iter()
            .find(|sm| sm.return_data)
            .map(|sm| sm.id.clone())
            .or_else(|| {
                sub_metrics
                    .iter()
                    .find(|sm| sm.expression.is_some())
                    .map(|sm| sm.id.clone())
            });

        // Build MetricDataQuery list from stored sub_metrics
        let queries: Vec<MetricDataQuery> = sub_metrics
            .iter()
            .filter_map(|sm| {
                let force_return = visible_id.as_deref() == Some(sm.id.as_str());
                let mut b = MetricDataQuery::builder().id(&sm.id);
                b = b.return_data(sm.return_data || force_return);
                if let Some(expr) = &sm.expression {
                    b = b.expression(expr);
                    // Both metric math and Metrics Insights expressions require period.
                    // Use the period stored on this SubMetric (read from describe_alarms),
                    // falling back to default_period from sibling entries, or sensible defaults.
                    let is_insights_query = expr.trim_start().to_uppercase().starts_with("SELECT");
                    let p = if sm.period > 0 {
                        sm.period
                    } else if is_insights_query {
                        default_period.unwrap_or(300)
                    } else {
                        default_period.unwrap_or(60)
                    };
                    b = b.period(p);
                } else if let (Some(ns), Some(mn)) = (&sm.namespace, &sm.metric_name) {
                    if ns.is_empty() || mn.is_empty() {
                        return None;
                    }
                    let mut metric_b = Metric::builder().namespace(ns).metric_name(mn);
                    for (k, v) in &sm.dimensions {
                        metric_b =
                            metric_b.dimensions(Dimension::builder().name(k).value(v).build());
                    }
                    let metric = metric_b.build();
                    let ms = MetricStat::builder()
                        .metric(metric)
                        .period(sm.period)
                        .stat(&sm.stat)
                        .build();
                    b = b.metric_stat(ms);
                } else {
                    return None;
                }
                Some(b.build())
            })
            .collect();

        if queries.is_empty() {
            return Ok(vec![]);
        }

        let mut req = client
            .get_metric_data()
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                end_time.timestamp_millis(),
            ));
        for q in queries {
            req = req.metric_data_queries(q);
        }

        let resp = req.send().await?;

        // Find the result for the visible entry
        let result = resp
            .metric_data_results()
            .iter()
            .find(|r| visible_id.as_deref() == r.id())
            .or_else(|| {
                resp.metric_data_results().iter().find(|r| {
                    sub_metrics
                        .iter()
                        .any(|sm| sm.return_data && sm.id == r.id().unwrap_or(""))
                })
            })
            .or_else(|| resp.metric_data_results().first());

        let mut data: Vec<(i64, f64)> = match result {
            Some(r) => r
                .timestamps()
                .iter()
                .zip(r.values().iter())
                .map(|(ts, val)| (ts.as_secs_f64() as i64, *val))
                .collect(),
            None => vec![],
        };
        data.sort_by_key(|(ts, _)| *ts);
        Ok(data)
    }

    pub async fn get_metric_statistics(
        &self,
        req: MetricStatsRequest<'_>,
    ) -> Result<Vec<(i64, f64)>> {
        use aws_sdk_cloudwatch::types::Dimension;
        let client = self.config.cloudwatch_client();
        let MetricStatsRequest {
            namespace,
            metric_name,
            statistic,
            period,
            dimensions,
            start_time,
            end_time,
        } = req;

        let mut cw_req = client
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
            ));

        for (name, value) in dimensions {
            cw_req = cw_req.dimensions(Dimension::builder().name(name).value(value).build());
        }

        let resp = cw_req.send().await?;

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
