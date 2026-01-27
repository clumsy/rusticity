pub mod apig;
pub mod app;
pub mod aws;
pub mod cfn;
pub mod common;
pub mod cw;
pub mod ec2;
pub mod ecr;
pub mod event;
pub mod iam;
pub mod keymap;
pub mod lambda;
pub mod s3;
pub mod session;
pub mod sqs;
pub mod table;
pub mod ui;

pub use app::{
    App, DetailTab, EventColumn, EventFilterFocus, LogGroupColumn, Service, StreamSort, ViewMode,
};
pub use cw::insights::{DateRangeType, InsightsFocus, InsightsState, QueryLanguage, TimeUnit};
use std::collections::HashMap;

/// Initialize all services (i18n, etc.)
pub fn init() {
    // Load column customizations from config.toml
    let mut i18n = HashMap::new();
    if let Some(home) = std::env::var_os("HOME") {
        let config_path = std::path::Path::new(&home)
            .join(".config")
            .join("rusticity")
            .join("config.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(toml_map) = contents.parse::<toml::Table>() {
                if let Some(columns_section) = toml_map.get("columns").and_then(|v| v.as_table()) {
                    flatten_toml(columns_section, "column", &mut i18n);
                }
            }
        }
    }

    // Initialize each service to populate their column defaults
    apig::init(&mut i18n);
    lambda::init(&mut i18n);
    ec2::init(&mut i18n);
    ecr::init(&mut i18n);
    s3::init(&mut i18n);
    cw::init(&mut i18n);
    sqs::init(&mut i18n);
    cfn::init(&mut i18n);
    iam::init(&mut i18n);

    // Set shared i18n map after all defaults are added
    common::set_i18n(i18n);
}

fn flatten_toml(
    table: &toml::Table,
    prefix: &str,
    map: &mut std::collections::HashMap<String, String>,
) {
    for (key, value) in table {
        let full_key = format!("{}.{}", prefix, key);
        match value {
            toml::Value::String(s) => {
                map.insert(full_key, s.clone());
            }
            toml::Value::Table(t) => {
                flatten_toml(t, &full_key, map);
            }
            _ => {}
        }
    }
}

pub use event::EventHandler;
pub use session::Session;
