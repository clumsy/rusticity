//! Data migrations tab: AWS DMS homogeneous data migrations that use this DB
//! instance as source or target. DMS links a migration to a database only
//! through its migration project and data providers, so the match is the
//! data provider whose server name is this instance endpoint. Read-only:
//! DescribeDataProviders, DescribeMigrationProjects, DescribeDataMigrations.
use crate::config::AwsConfig;
use anyhow::{bail, Result};
use aws_sdk_databasemigration::types::{
    DataMigration, DataProvider, DataProviderSettings, MigrationProject,
};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsDataMigration {
    pub name: String,
    pub status: String,
    /// Full load percentage, `45%`; empty without statistics.
    pub progress: String,
    /// Raw `DataMigrationType`: full-load, cdc, full-load-and-cdc.
    pub migration_type: String,
    pub server_name: String,
    pub port: Option<i32>,
    pub cloudwatch_logs: Option<bool>,
    pub created_time: String,
    pub started_time: String,
    pub stopped_time: String,
    pub tables_loaded: Option<i32>,
    pub tables_loading: Option<i32>,
    pub tables_queued: Option<i32>,
    pub tables_errored: Option<i32>,
    pub elapsed_millis: Option<i64>,
    pub arn: String,
}

/// Server name and port of a data provider, for every engine that has them.
fn server(settings: Option<&DataProviderSettings>) -> Option<(&str, Option<i32>)> {
    use DataProviderSettings as S;
    Some(match settings? {
        S::PostgreSqlSettings(s) => (s.server_name()?, s.port()),
        S::MySqlSettings(s) => (s.server_name()?, s.port()),
        S::MariaDbSettings(s) => (s.server_name()?, s.port()),
        S::OracleSettings(s) => (s.server_name()?, s.port()),
        S::MicrosoftSqlServerSettings(s) => (s.server_name()?, s.port()),
        S::IbmDb2LuwSettings(s) => (s.server_name()?, s.port()),
        _ => return None,
    })
}

pub(super) async fn read(config: &AwsConfig, endpoint: &str) -> Result<Vec<RdsDataMigration>> {
    if endpoint.is_empty() {
        bail!("the instance has no endpoint yet");
    }
    let client = config.dms_client();
    let providers: Vec<DataProvider> = client
        .describe_data_providers()
        .into_paginator()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?
        .iter()
        .flat_map(|page| page.data_providers().to_vec())
        .collect();
    // provider ARN -> (server, port) for providers that point at this endpoint.
    let own: HashMap<String, (String, Option<i32>)> = providers
        .iter()
        .filter_map(|p| {
            let (name, port) = server(p.settings())?;
            if !name.eq_ignore_ascii_case(endpoint) {
                return None;
            }
            Some((p.data_provider_arn()?.to_string(), (name.to_string(), port)))
        })
        .collect();
    if own.is_empty() {
        return Ok(Vec::new());
    }
    let projects: Vec<MigrationProject> = client
        .describe_migration_projects()
        .into_paginator()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?
        .iter()
        .flat_map(|page| page.migration_projects().to_vec())
        .collect();
    let project_server = project_servers(&projects, &own);
    if project_server.is_empty() {
        return Ok(Vec::new());
    }
    let migrations = client
        .describe_data_migrations()
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    Ok(own_migrations(&migrations, &project_server))
}

/// project ARN -> the matched provider (target first, as DMS writes there).
pub(super) fn project_servers(
    projects: &[MigrationProject],
    own: &HashMap<String, (String, Option<i32>)>,
) -> HashMap<String, (String, Option<i32>)> {
    projects
        .iter()
        .filter_map(|p| {
            let found = p
                .target_data_provider_descriptors()
                .iter()
                .chain(p.source_data_provider_descriptors())
                .find_map(|d| own.get(d.data_provider_arn()?))?;
            Some((p.migration_project_arn()?.to_string(), found.clone()))
        })
        .collect()
}

pub(super) fn own_migrations(
    migrations: &[DataMigration],
    project_server: &HashMap<String, (String, Option<i32>)>,
) -> Vec<RdsDataMigration> {
    let time = |t| super::super::timestamp(t);
    let mut rows: Vec<_> = migrations
        .iter()
        .filter_map(|m| {
            let (server_name, port) = project_server.get(m.migration_project_arn()?)?.clone();
            let stats = m.data_migration_statistics();
            Some(RdsDataMigration {
                name: m.data_migration_name().unwrap_or_default().into(),
                status: m.data_migration_status().unwrap_or_default().into(),
                progress: stats
                    .map(|s| format!("{}%", s.full_load_percentage()))
                    .unwrap_or_default(),
                migration_type: m
                    .data_migration_type()
                    .map(|t| t.as_str().to_string())
                    .unwrap_or_default(),
                server_name,
                port,
                cloudwatch_logs: m
                    .data_migration_settings()
                    .and_then(|s| s.cloudwatch_logs_enabled()),
                created_time: time(m.data_migration_create_time()),
                started_time: time(
                    m.data_migration_start_time()
                        .or(stats.and_then(|s| s.start_time())),
                ),
                stopped_time: time(
                    m.data_migration_end_time()
                        .or(stats.and_then(|s| s.stop_time())),
                ),
                tables_loaded: stats.map(|s| s.tables_loaded()),
                tables_loading: stats.map(|s| s.tables_loading()),
                tables_queued: stats.map(|s| s.tables_queued()),
                tables_errored: stats.map(|s| s.tables_errored()),
                elapsed_millis: stats.map(|s| s.elapsed_time_millis()),
                arn: m.data_migration_arn().unwrap_or_default().into(),
            })
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}
