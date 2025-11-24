use crate::common::{format_iso_timestamp, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;
use std::sync::OnceLock;

static I18N: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn init() {
    let mut map = HashMap::new();

    if let Some(home) = std::env::var_os("HOME") {
        let config_path = std::path::Path::new(&home)
            .join(".config")
            .join("rusticity")
            .join("i18n.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(toml_map) = contents.parse::<toml::Table>() {
                if let Some(column_section) = toml_map.get("column").and_then(|v| v.as_table()) {
                    flatten_toml(column_section, "column", &mut map);
                }
            }
        }
    }

    // Set defaults from enum
    for col in [
        Column::Tag,
        Column::ArtifactType,
        Column::PushedAt,
        Column::SizeMb,
        Column::Uri,
        Column::Digest,
        Column::LastPullTime,
    ] {
        let key = format!("column.ecr.image.{}", col.id());
        map.entry(key)
            .or_insert_with(|| col.default_name().to_string());
    }

    I18N.set(map).ok();
}

fn flatten_toml(table: &toml::Table, prefix: &str, map: &mut HashMap<String, String>) {
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

fn t(key: &str) -> String {
    I18N.get()
        .and_then(|map| map.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

#[derive(Debug, Clone)]
pub struct Image {
    pub tag: String,
    pub artifact_type: String,
    pub pushed_at: String,
    pub size_bytes: i64,
    pub uri: String,
    pub digest: String,
    pub last_pull_time: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Tag,
    ArtifactType,
    PushedAt,
    SizeMb,
    Uri,
    Digest,
    LastPullTime,
}

impl Column {
    pub fn id(&self) -> &'static str {
        match self {
            Column::Tag => "tag",
            Column::ArtifactType => "artifact_type",
            Column::PushedAt => "pushed_at",
            Column::SizeMb => "size_mb",
            Column::Uri => "uri",
            Column::Digest => "digest",
            Column::LastPullTime => "last_pull_time",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::Tag => "Image tag",
            Column::ArtifactType => "Artifact type",
            Column::PushedAt => "Pushed at",
            Column::SizeMb => "Size (MB)",
            Column::Uri => "Image URI",
            Column::Digest => "Digest",
            Column::LastPullTime => "Last recorded pull time",
        }
    }

    pub fn all() -> Vec<ColumnId> {
        [
            Column::Tag,
            Column::ArtifactType,
            Column::PushedAt,
            Column::SizeMb,
            Column::Uri,
            Column::Digest,
            Column::LastPullTime,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "tag" => Some(Column::Tag),
            "artifact_type" => Some(Column::ArtifactType),
            "pushed_at" => Some(Column::PushedAt),
            "size_mb" => Some(Column::SizeMb),
            "uri" => Some(Column::Uri),
            "digest" => Some(Column::Digest),
            "last_pull_time" => Some(Column::LastPullTime),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.ecr.image.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }
}

impl TableColumn<Image> for Column {
    fn name(&self) -> &str {
        let key = format!("column.ecr.image.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.ecr.image.{}", self.id()));
        translated.len().max(match self {
            Column::Tag => 20,
            Column::ArtifactType => 20,
            Column::PushedAt => UTC_TIMESTAMP_WIDTH as usize,
            Column::SizeMb => 12,
            Column::Uri => 50,
            Column::Digest => 20,
            Column::LastPullTime => UTC_TIMESTAMP_WIDTH as usize,
        }) as u16
    }

    fn render(&self, item: &Image) -> (String, Style) {
        let text = match self {
            Column::Tag => item.tag.clone(),
            Column::ArtifactType => item.artifact_type.clone(),
            Column::PushedAt => format_iso_timestamp(&item.pushed_at),
            Column::SizeMb => crate::common::format_bytes(item.size_bytes),
            Column::Uri => item.uri.clone(),
            Column::Digest => item.digest.clone(),
            Column::LastPullTime => format_iso_timestamp(&item.last_pull_time),
        };
        (text, Style::default())
    }
}
