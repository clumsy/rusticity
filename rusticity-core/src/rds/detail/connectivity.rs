//! Connectivity & security tables beyond DescribeDBInstances: connected
//! compute resources, security group rules and replication. Read-only EC2,
//! Lambda, RDS and CloudWatch reads; each table fails on its own.
use crate::config::AwsConfig;
use crate::metrics::{MetricQuery, MetricReader};
use anyhow::Result;
use aws_sdk_cloudwatch::types::Statistic;
use aws_sdk_ec2::types::{Filter, SecurityGroup, SecurityGroupRule};
use aws_sdk_rds::types::DbInstance;
use std::collections::HashMap;

/// An EC2 instance or Lambda function connected through the security group
/// pair the console creates (`rds-ec2-N` on the database, `ec2-rds-N` on the
/// compute resource; `rds-lambda-N` and `lambda-rds-N` for Lambda).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsComputeResource {
    pub identifier: String,
    pub resource_type: String,
    pub availability_zone: String,
    pub db_security_group: String,
    pub compute_security_group: String,
    /// Not read: always empty.
    pub proxy: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsSecurityGroupRule {
    /// `default (sg-123)`.
    pub security_group: String,
    /// `CIDR/IP - Inbound`, `EC2 Security Group - Outbound`, ...
    pub rule_type: String,
    pub rule: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsReplica {
    pub identifier: String,
    pub role: String,
    pub availability_zone: String,
    pub source: String,
    /// Raw `read replication` status, `replicating`.
    pub state: String,
    pub lag: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsIamRole {
    pub role_arn: String,
    pub feature: String,
    pub status: String,
}

pub(super) fn iam_roles(i: &DbInstance) -> Vec<RdsIamRole> {
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    i.associated_roles()
        .iter()
        .map(|r| RdsIamRole {
            role_arn: text(r.role_arn()),
            feature: text(r.feature_name()),
            status: text(r.status()),
        })
        .collect()
}

fn label(group: &SecurityGroup) -> String {
    let id = group.group_id().unwrap_or_default();
    match group.group_name() {
        Some(name) if !name.is_empty() => format!("{name} ({id})"),
        _ => id.to_string(),
    }
}

async fn groups(config: &AwsConfig, ids: &[String]) -> Result<Vec<SecurityGroup>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(config
        .ec2_client()
        .describe_security_groups()
        .set_group_ids(Some(ids.to_vec()))
        .send()
        .await?
        .security_groups()
        .to_vec())
}

async fn rules(config: &AwsConfig, ids: &[String]) -> Result<Vec<SecurityGroupRule>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let pages = config
        .ec2_client()
        .describe_security_group_rules()
        .filters(
            Filter::builder()
                .name("group-id")
                .set_values(Some(ids.to_vec()))
                .build(),
        )
        .into_paginator()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    Ok(pages
        .iter()
        .flat_map(|p| p.security_group_rules().to_vec())
        .collect())
}

/// The database security groups, ids only (`sg-1 (active)` -> `sg-1`).
pub(super) fn group_ids(security_groups: &[String]) -> Vec<String> {
    security_groups
        .iter()
        .filter_map(|g| g.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

pub(super) async fn security_group_rules(
    config: &AwsConfig,
    security_groups: &[String],
) -> Result<Vec<RdsSecurityGroupRule>> {
    let ids = group_ids(security_groups);
    let (groups, rules) = tokio::try_join!(groups(config, &ids), rules(config, &ids))?;
    Ok(rule_rows(&groups, &rules))
}

pub(super) fn rule_rows(
    groups: &[SecurityGroup],
    rules: &[SecurityGroupRule],
) -> Vec<RdsSecurityGroupRule> {
    let labels: HashMap<&str, String> = groups
        .iter()
        .filter_map(|g| Some((g.group_id()?, label(g))))
        .collect();
    let mut rows: Vec<_> = rules
        .iter()
        .map(|r| {
            let id = r.group_id().unwrap_or_default();
            let direction = if r.is_egress() == Some(true) {
                "Outbound"
            } else {
                "Inbound"
            };
            let (kind, rule) = if let Some(cidr) = r.cidr_ipv4().or(r.cidr_ipv6()) {
                ("CIDR/IP", cidr.to_string())
            } else if let Some(group) = r.referenced_group_info().and_then(|g| g.group_id()) {
                ("EC2 Security Group", group.to_string())
            } else if let Some(list) = r.prefix_list_id() {
                ("Prefix list", list.to_string())
            } else {
                ("", String::new())
            };
            RdsSecurityGroupRule {
                security_group: labels.get(id).cloned().unwrap_or_else(|| id.to_string()),
                rule_type: if kind.is_empty() {
                    direction.into()
                } else {
                    format!("{kind} - {direction}")
                },
                rule,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        (&a.security_group, &a.rule_type, &a.rule).cmp(&(&b.security_group, &b.rule_type, &b.rule))
    });
    rows
}

/// Compute side group prefix for a console-created database group.
fn compute_prefix(db_group_name: &str) -> Option<(&'static str, &'static str)> {
    if db_group_name.starts_with("rds-ec2-") {
        Some(("ec2-rds-", "EC2 instance"))
    } else if db_group_name.starts_with("rds-lambda-") {
        Some(("lambda-rds-", "Lambda function"))
    } else {
        None
    }
}

/// (database group, compute group, resource type) pairs.
pub(super) fn pairs(
    db_groups: &[SecurityGroup],
    rules: &[SecurityGroupRule],
    referenced: &[SecurityGroup],
) -> Vec<(String, String, &'static str)> {
    let names: HashMap<&str, &str> = db_groups
        .iter()
        .chain(referenced)
        .filter_map(|g| Some((g.group_id()?, g.group_name()?)))
        .collect();
    let mut out = Vec::new();
    for r in rules.iter().filter(|r| r.is_egress() != Some(true)) {
        let (Some(db), Some(compute)) = (
            r.group_id(),
            r.referenced_group_info().and_then(|g| g.group_id()),
        ) else {
            continue;
        };
        let Some((prefix, kind)) = names.get(db).and_then(|n| compute_prefix(n)) else {
            continue;
        };
        if names.get(compute).is_some_and(|n| n.starts_with(prefix))
            && !out.iter().any(|(d, c, _)| d == db && c == compute)
        {
            out.push((db.to_string(), compute.to_string(), kind));
        }
    }
    out
}

pub(super) async fn compute_resources(
    config: &AwsConfig,
    security_groups: &[String],
) -> Result<Vec<RdsComputeResource>> {
    let ids = group_ids(security_groups);
    let (db_groups, rules) = tokio::try_join!(groups(config, &ids), rules(config, &ids))?;
    let referenced_ids: Vec<String> = rules
        .iter()
        .filter_map(|r| r.referenced_group_info()?.group_id())
        .filter(|id| !ids.iter().any(|own| own == id))
        .map(str::to_string)
        .collect();
    let referenced = groups(config, &referenced_ids).await?;
    let pairs = pairs(&db_groups, &rules, &referenced);
    let label_of = |id: &str| {
        db_groups
            .iter()
            .chain(&referenced)
            .find(|g| g.group_id() == Some(id))
            .map(label)
            .unwrap_or_else(|| id.to_string())
    };
    let mut rows = Vec::new();
    let ec2: Vec<_> = pairs.iter().filter(|p| p.2 == "EC2 instance").collect();
    if !ec2.is_empty() {
        let pages = config
            .ec2_client()
            .describe_instances()
            .filters(
                Filter::builder()
                    .name("instance.group-id")
                    .set_values(Some(ec2.iter().map(|p| p.1.clone()).collect()))
                    .build(),
            )
            .into_paginator()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?;
        for instance in pages
            .iter()
            .flat_map(|p| p.reservations())
            .flat_map(|r| r.instances())
        {
            for (db, compute, kind) in &ec2 {
                if instance
                    .security_groups()
                    .iter()
                    .any(|g| g.group_id() == Some(compute.as_str()))
                {
                    rows.push(RdsComputeResource {
                        identifier: instance.instance_id().unwrap_or_default().into(),
                        resource_type: kind.to_string(),
                        availability_zone: instance
                            .placement()
                            .and_then(|p| p.availability_zone())
                            .unwrap_or_default()
                            .into(),
                        db_security_group: label_of(db),
                        compute_security_group: label_of(compute),
                        proxy: String::new(),
                    });
                }
            }
        }
    }
    let lambda: Vec<_> = pairs.iter().filter(|p| p.2 == "Lambda function").collect();
    if !lambda.is_empty() {
        let functions = config
            .lambda_client()
            .list_functions()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?;
        for function in &functions {
            let groups = function
                .vpc_config()
                .map(|v| v.security_group_ids())
                .unwrap_or_default();
            for (db, compute, kind) in &lambda {
                if groups.iter().any(|g| g == compute) {
                    rows.push(RdsComputeResource {
                        identifier: function.function_name().unwrap_or_default().into(),
                        resource_type: kind.to_string(),
                        availability_zone: String::new(),
                        db_security_group: label_of(db),
                        compute_security_group: label_of(compute),
                        proxy: String::new(),
                    });
                }
            }
        }
    }
    rows.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    Ok(rows)
}

fn replication_state(i: &DbInstance) -> String {
    i.status_infos()
        .iter()
        .find(|s| s.status_type() == Some("read replication"))
        .and_then(|s| s.status())
        .unwrap_or_default()
        .into()
}

/// This instance, its source and its same-Region replicas. Cross-Region
/// members (ARNs) are listed by identity only.
pub(super) async fn replication(config: &AwsConfig, own: &DbInstance) -> Result<Vec<RdsReplica>> {
    let identifier = own.db_instance_identifier().unwrap_or_default();
    let source = own
        .read_replica_source_db_instance_identifier()
        .unwrap_or_default();
    let replicas = own.read_replica_db_instance_identifiers();
    let local: Vec<String> = replicas
        .iter()
        .filter(|r| !r.starts_with("arn:"))
        .cloned()
        .collect();
    let described: Vec<DbInstance> = if local.is_empty() {
        Vec::new()
    } else {
        config
            .rds_client()
            .describe_db_instances()
            .filters(
                aws_sdk_rds::types::Filter::builder()
                    .name("db-instance-id")
                    .set_values(Some(local.clone()))
                    .build(),
            )
            .send()
            .await?
            .db_instances()
            .to_vec()
    };
    let reader = MetricReader::new(config.cloudwatch_client());
    let lag = |id: String, aurora: bool| {
        let reader = &reader;
        async move {
            let metric = if aurora {
                "AuroraReplicaLag"
            } else {
                "ReplicaLag"
            };
            let query = MetricQuery::new("AWS/RDS", metric, Statistic::Average)
                .dimension("DBInstanceIdentifier", &id);
            match reader.recent(query, 60).await {
                Ok(points) => match points.last() {
                    Some((_, v)) if *v >= 0.0 => format!("{v:.0} seconds"),
                    _ => String::new(),
                },
                Err(_) => "Unavailable".into(),
            }
        }
    };
    let aurora = own.engine().unwrap_or_default().starts_with("aurora");
    let mut rows = vec![RdsReplica {
        identifier: identifier.into(),
        role: if !source.is_empty() {
            "Replica".into()
        } else if !replicas.is_empty() {
            "Primary".into()
        } else {
            "Instance".into()
        },
        availability_zone: own.availability_zone().unwrap_or_default().into(),
        source: source.into(),
        state: replication_state(own),
        lag: if source.is_empty() {
            String::new()
        } else {
            lag(identifier.into(), aurora).await
        },
    }];
    for replica in replicas {
        let found = described
            .iter()
            .find(|d| d.db_instance_identifier() == Some(replica.as_str()));
        rows.push(RdsReplica {
            identifier: replica.clone(),
            role: "Replica".into(),
            availability_zone: found
                .and_then(|d| d.availability_zone())
                .unwrap_or_default()
                .into(),
            source: identifier.into(),
            state: found.map(replication_state).unwrap_or_default(),
            lag: if found.is_some() {
                lag(replica.clone(), aurora).await
            } else {
                String::new()
            },
        });
    }
    Ok(rows)
}
