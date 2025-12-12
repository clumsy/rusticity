use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Instance {
    pub instance_id: String,
    pub name: String,
    pub state: String,
    pub instance_type: String,
    pub availability_zone: String,
    pub public_ipv4_dns: String,
    pub public_ipv4_address: String,
    pub elastic_ip: String,
    pub ipv6_ips: String,
    pub monitoring: String,
    pub security_groups: String,
    pub key_name: String,
    pub launch_time: String,
    pub platform_details: String,
    pub status_checks: String,
    pub alarm_status: String,
}

#[derive(Clone, Debug)]
pub struct InstanceTag {
    pub key: String,
    pub value: String,
}

pub struct Ec2Client {
    config: AwsConfig,
}

impl Ec2Client {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_instances(&self) -> Result<Vec<Instance>> {
        let client = self.config.ec2_client().await;
        let mut instances = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.describe_instances();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(reservations) = response.reservations {
                for reservation in reservations {
                    if let Some(insts) = reservation.instances {
                        for inst in insts {
                            let tags: std::collections::HashMap<String, String> = inst
                                .tags()
                                .iter()
                                .filter_map(|t| {
                                    Some((t.key()?.to_string(), t.value()?.to_string()))
                                })
                                .collect();

                            let name = tags.get("Name").cloned().unwrap_or_default();

                            let state = inst
                                .state()
                                .and_then(|s| s.name())
                                .map(|n| n.as_str().to_string())
                                .unwrap_or_default();

                            let security_groups = inst
                                .security_groups()
                                .iter()
                                .filter_map(|sg| sg.group_name())
                                .collect::<Vec<_>>()
                                .join(", ");

                            let ipv6_ips = inst
                                .network_interfaces()
                                .iter()
                                .flat_map(|ni| ni.ipv6_addresses())
                                .filter_map(|ip| ip.ipv6_address())
                                .collect::<Vec<_>>()
                                .join(", ");

                            let launch_time = inst
                                .launch_time()
                                .map(|dt| {
                                    let timestamp = dt.secs();
                                    chrono::DateTime::from_timestamp(timestamp, 0)
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S (UTC)").to_string())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();

                            instances.push(Instance {
                                instance_id: inst.instance_id().unwrap_or("").to_string(),
                                name,
                                state,
                                instance_type: inst
                                    .instance_type()
                                    .map(|t| t.as_str().to_string())
                                    .unwrap_or_default(),
                                availability_zone: inst
                                    .placement()
                                    .and_then(|p| p.availability_zone())
                                    .unwrap_or("")
                                    .to_string(),
                                public_ipv4_dns: inst.public_dns_name().unwrap_or("").to_string(),
                                public_ipv4_address: inst
                                    .public_ip_address()
                                    .unwrap_or("")
                                    .to_string(),
                                elastic_ip: String::new(),
                                ipv6_ips,
                                monitoring: inst
                                    .monitoring()
                                    .and_then(|m| m.state())
                                    .map(|s| s.as_str().to_string())
                                    .unwrap_or_default(),
                                security_groups,
                                key_name: inst.key_name().unwrap_or("").to_string(),
                                launch_time,
                                platform_details: inst.platform_details().unwrap_or("").to_string(),
                                status_checks: String::new(),
                                alarm_status: String::new(),
                            });
                        }
                    }
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(instances)
    }

    pub async fn list_tags(&self, instance_id: &str) -> Result<Vec<InstanceTag>> {
        let client = self.config.ec2_client().await;

        let response = client
            .describe_tags()
            .filters(
                aws_sdk_ec2::types::Filter::builder()
                    .name("resource-id")
                    .values(instance_id)
                    .build(),
            )
            .send()
            .await?;

        let mut tags = Vec::new();
        if let Some(tag_list) = response.tags {
            for tag in tag_list {
                if let (Some(key), Some(value)) = (tag.key, tag.value) {
                    tags.push(InstanceTag { key, value });
                }
            }
        }

        Ok(tags)
    }

    pub async fn get_cpu_metrics(&self, instance_id: &str) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client().await;
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::hours(3);

        let response = client
            .get_metric_statistics()
            .namespace("AWS/EC2")
            .metric_name("CPUUtilization")
            .dimensions(
                aws_sdk_cloudwatch::types::Dimension::builder()
                    .name("InstanceId")
                    .value(instance_id)
                    .build(),
            )
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                now.timestamp_millis(),
            ))
            .period(300)
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await?;

        let mut data_points = Vec::new();
        if let Some(datapoints) = response.datapoints {
            for dp in datapoints {
                if let (Some(timestamp), Some(value)) = (dp.timestamp, dp.average) {
                    data_points.push((timestamp.secs(), value));
                }
            }
        }

        data_points.sort_by_key(|(ts, _)| *ts);
        Ok(data_points)
    }

    pub async fn get_network_in_metrics(&self, instance_id: &str) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client().await;
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::hours(3);

        let response = client
            .get_metric_statistics()
            .namespace("AWS/EC2")
            .metric_name("NetworkIn")
            .dimensions(
                aws_sdk_cloudwatch::types::Dimension::builder()
                    .name("InstanceId")
                    .value(instance_id)
                    .build(),
            )
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                now.timestamp_millis(),
            ))
            .period(300)
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await?;

        let mut data_points = Vec::new();
        if let Some(datapoints) = response.datapoints {
            for dp in datapoints {
                if let (Some(timestamp), Some(value)) = (dp.timestamp, dp.average) {
                    data_points.push((timestamp.secs(), value));
                }
            }
        }

        data_points.sort_by_key(|(ts, _)| *ts);
        Ok(data_points)
    }

    pub async fn get_network_out_metrics(&self, instance_id: &str) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client().await;
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::hours(3);

        let response = client
            .get_metric_statistics()
            .namespace("AWS/EC2")
            .metric_name("NetworkOut")
            .dimensions(
                aws_sdk_cloudwatch::types::Dimension::builder()
                    .name("InstanceId")
                    .value(instance_id)
                    .build(),
            )
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                now.timestamp_millis(),
            ))
            .period(300)
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await?;

        let mut data_points = Vec::new();
        if let Some(datapoints) = response.datapoints {
            for dp in datapoints {
                if let (Some(timestamp), Some(value)) = (dp.timestamp, dp.average) {
                    data_points.push((timestamp.secs(), value));
                }
            }
        }

        data_points.sort_by_key(|(ts, _)| *ts);
        Ok(data_points)
    }

    pub async fn get_network_packets_in_metrics(
        &self,
        instance_id: &str,
    ) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client().await;
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::hours(3);

        let response = client
            .get_metric_statistics()
            .namespace("AWS/EC2")
            .metric_name("NetworkPacketsIn")
            .dimensions(
                aws_sdk_cloudwatch::types::Dimension::builder()
                    .name("InstanceId")
                    .value(instance_id)
                    .build(),
            )
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                now.timestamp_millis(),
            ))
            .period(300)
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await?;

        let mut data_points = Vec::new();
        if let Some(datapoints) = response.datapoints {
            for dp in datapoints {
                if let (Some(timestamp), Some(value)) = (dp.timestamp, dp.average) {
                    data_points.push((timestamp.secs(), value));
                }
            }
        }

        data_points.sort_by_key(|(ts, _)| *ts);
        Ok(data_points)
    }

    pub async fn get_network_packets_out_metrics(
        &self,
        instance_id: &str,
    ) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client().await;
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::hours(3);

        let response = client
            .get_metric_statistics()
            .namespace("AWS/EC2")
            .metric_name("NetworkPacketsOut")
            .dimensions(
                aws_sdk_cloudwatch::types::Dimension::builder()
                    .name("InstanceId")
                    .value(instance_id)
                    .build(),
            )
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                now.timestamp_millis(),
            ))
            .period(300)
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await?;

        let mut data_points = Vec::new();
        if let Some(datapoints) = response.datapoints {
            for dp in datapoints {
                if let (Some(timestamp), Some(value)) = (dp.timestamp, dp.average) {
                    data_points.push((timestamp.secs(), value));
                }
            }
        }

        data_points.sort_by_key(|(ts, _)| *ts);
        Ok(data_points)
    }

    pub async fn get_metadata_no_token_metrics(
        &self,
        instance_id: &str,
    ) -> Result<Vec<(i64, f64)>> {
        let client = self.config.cloudwatch_client().await;
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::hours(3);

        let response = client
            .get_metric_statistics()
            .namespace("AWS/EC2")
            .metric_name("MetadataNoToken")
            .dimensions(
                aws_sdk_cloudwatch::types::Dimension::builder()
                    .name("InstanceId")
                    .value(instance_id)
                    .build(),
            )
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                now.timestamp_millis(),
            ))
            .period(300)
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await?;

        let mut data_points = Vec::new();
        if let Some(datapoints) = response.datapoints {
            for dp in datapoints {
                if let (Some(timestamp), Some(value)) = (dp.timestamp, dp.average) {
                    data_points.push((timestamp.secs(), value));
                }
            }
        }

        data_points.sort_by_key(|(ts, _)| *ts);
        Ok(data_points)
    }
}
