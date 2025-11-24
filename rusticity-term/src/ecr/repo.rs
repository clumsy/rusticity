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
        Column::Name,
        Column::Uri,
        Column::CreatedAt,
        Column::TagImmutability,
        Column::EncryptionType,
    ] {
        let key = format!("column.ecr.repo.{}", col.id());
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
pub struct Repository {
    pub name: String,
    pub uri: String,
    pub created_at: String,
    pub tag_immutability: String,
    pub encryption_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    Uri,
    CreatedAt,
    TagImmutability,
    EncryptionType,
}

impl Column {
    pub fn id(&self) -> &'static str {
        match self {
            Column::Name => "name",
            Column::Uri => "uri",
            Column::CreatedAt => "created_at",
            Column::TagImmutability => "tag_immutability",
            Column::EncryptionType => "encryption_type",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Repository name",
            Column::Uri => "URI",
            Column::CreatedAt => "Created at",
            Column::TagImmutability => "Tag immutability",
            Column::EncryptionType => "Encryption type",
        }
    }

    pub fn all() -> Vec<ColumnId> {
        [
            Column::Name,
            Column::Uri,
            Column::CreatedAt,
            Column::TagImmutability,
            Column::EncryptionType,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "name" => Some(Column::Name),
            "uri" => Some(Column::Uri),
            "created_at" => Some(Column::CreatedAt),
            "tag_immutability" => Some(Column::TagImmutability),
            "encryption_type" => Some(Column::EncryptionType),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.ecr.repo.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }
}

impl TableColumn<Repository> for Column {
    fn name(&self) -> &str {
        let key = format!("column.ecr.repo.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.ecr.repo.{}", self.id()));
        translated.len().max(match self {
            Column::Name => 30,
            Column::Uri => 50,
            Column::CreatedAt => UTC_TIMESTAMP_WIDTH as usize,
            Column::TagImmutability => 18,
            Column::EncryptionType => 18,
        }) as u16
    }

    fn render(&self, item: &Repository) -> (String, Style) {
        let text = match self {
            Column::Name => item.name.clone(),
            Column::Uri => item.uri.clone(),
            Column::CreatedAt => format_iso_timestamp(&item.created_at),
            Column::TagImmutability => item.tag_immutability.clone(),
            Column::EncryptionType => match item.encryption_type.as_str() {
                "AES256" => "AES-256".to_string(),
                "KMS" => "KMS".to_string(),
                other => other.to_string(),
            },
        };
        (text, Style::default())
    }
}
