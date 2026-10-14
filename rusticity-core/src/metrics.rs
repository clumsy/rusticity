//! Shared CloudWatch GetMetricStatistics request/response plumbing.
//! GetMetricData (batched queries/expressions) deliberately remains separate.
use anyhow::Result;
use aws_sdk_cloudwatch::{
    operation::get_metric_statistics::builders::GetMetricStatisticsFluentBuilder,
    types::{Datapoint, Dimension, Statistic},
    Client,
};
use aws_smithy_types::DateTime;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MetricQuery<'a> {
    pub namespace: &'a str,
    pub name: &'a str,
    pub statistic: Statistic,
    pub dimensions: Vec<Dimension>,
}
impl<'a> MetricQuery<'a> {
    pub fn new(namespace: &'a str, name: &'a str, statistic: Statistic) -> Self {
        Self {
            namespace,
            name,
            statistic,
            dimensions: Vec::new(),
        }
    }
    pub fn dimension(mut self, name: &str, value: &str) -> Self {
        self.dimensions
            .push(Dimension::builder().name(name).value(value).build());
        self
    }
}

#[derive(Debug, Clone)]
pub struct MetricWindow {
    pub start: DateTime,
    pub end: DateTime,
    pub period: i32,
}
impl MetricWindow {
    pub fn last_three_hours(period: i32) -> Result<Self> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        Ok(Self {
            start: DateTime::from_secs(now - 3 * 3600),
            end: DateTime::from_secs(now),
            period,
        })
    }
}

pub struct MetricReader {
    client: Client,
}
impl MetricReader {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
    fn request(
        &self,
        query: &MetricQuery<'_>,
        window: MetricWindow,
    ) -> GetMetricStatisticsFluentBuilder {
        self.client
            .get_metric_statistics()
            .namespace(query.namespace)
            .metric_name(query.name)
            .statistics(query.statistic.clone())
            .set_dimensions(Some(query.dimensions.clone()))
            .start_time(window.start)
            .end_time(window.end)
            .period(window.period)
    }
    pub async fn read(
        &self,
        query: MetricQuery<'_>,
        window: MetricWindow,
    ) -> Result<Vec<(i64, f64)>> {
        let response = self.request(&query, window).send().await?;
        Ok(series(response.datapoints(), &query.statistic))
    }
    pub async fn recent(&self, query: MetricQuery<'_>, period: i32) -> Result<Vec<(i64, f64)>> {
        self.read(query, MetricWindow::last_three_hours(period)?)
            .await
    }
}

fn series(datapoints: &[Datapoint], statistic: &Statistic) -> Vec<(i64, f64)> {
    let mut points: Vec<_> = datapoints
        .iter()
        .filter_map(|point| {
            let timestamp = point.timestamp()?.secs();
            let value = match statistic {
                Statistic::Sum => point.sum(),
                Statistic::Minimum => point.minimum(),
                Statistic::Maximum => point.maximum(),
                Statistic::SampleCount => point.sample_count(),
                _ => point.average(),
            }?;
            Some((timestamp, value))
        })
        .collect();
    points.sort_by_key(|(time, _)| *time);
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AwsConfig;

    #[test]
    fn request_preserves_namespace_metric_dimensions_period_and_explicit_window() {
        let reader =
            MetricReader::new(AwsConfig::dummy(Some("us-east-2".into())).cloudwatch_client());
        let query = MetricQuery::new("AWS/Lambda", "Invocations", Statistic::Sum)
            .dimension("FunctionName", "function")
            .dimension("Resource", "function:alias");
        let request = reader.request(
            &query,
            MetricWindow {
                start: DateTime::from_secs(100),
                end: DateTime::from_secs(200),
                period: 60,
            },
        );
        let input = request.as_input();
        assert_eq!(input.get_namespace().as_deref(), Some("AWS/Lambda"));
        assert_eq!(input.get_metric_name().as_deref(), Some("Invocations"));
        assert_eq!(input.get_statistics().as_ref().unwrap(), &[Statistic::Sum]);
        assert_eq!(input.get_dimensions().as_ref().unwrap().len(), 2);
        assert_eq!(
            input.get_dimensions().as_ref().unwrap()[1].value(),
            Some("function:alias")
        );
        assert_eq!(input.get_period(), &Some(60));
        assert_eq!(input.get_start_time(), &Some(DateTime::from_secs(100)));
        assert_eq!(input.get_end_time(), &Some(DateTime::from_secs(200)));
    }

    #[test]
    fn recent_windows_use_one_clock_sample_and_preserve_period() {
        for period in [60, 300] {
            let window = MetricWindow::last_three_hours(period).unwrap();
            assert_eq!(window.end.secs() - window.start.secs(), 10800);
            assert_eq!(window.period, period);
        }
    }

    #[test]
    fn series_selects_each_statistic_sorts_and_does_not_invent_missing_values() {
        let complete = |time| {
            Datapoint::builder()
                .timestamp(DateTime::from_secs(time))
                .average(1.0)
                .sum(2.0)
                .minimum(0.0)
                .maximum(3.0)
                .sample_count(4.0)
                .build()
        };
        let points = [
            complete(20),
            Datapoint::builder().average(99.0).build(),
            Datapoint::builder()
                .timestamp(DateTime::from_secs(15))
                .build(),
            complete(10),
        ];
        for (stat, expected) in [
            (Statistic::Average, 1.0),
            (Statistic::Sum, 2.0),
            (Statistic::Minimum, 0.0),
            (Statistic::Maximum, 3.0),
            (Statistic::SampleCount, 4.0),
        ] {
            assert_eq!(series(&points, &stat), [(10, expected), (20, expected)]);
            assert!(series(&[], &stat).is_empty());
        }
    }
}
