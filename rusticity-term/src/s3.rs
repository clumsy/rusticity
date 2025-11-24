use crate::common::{format_bytes, ColumnId, UTC_TIMESTAMP_WIDTH};
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
        BucketColumn::Name,
        BucketColumn::Region,
        BucketColumn::CreationDate,
    ] {
        let key = format!("column.s3.bucket.{}", col.id());
        map.entry(key)
            .or_insert_with(|| col.default_name().to_string());
    }

    for col in [
        ObjectColumn::Key,
        ObjectColumn::Size,
        ObjectColumn::LastModified,
        ObjectColumn::StorageClass,
    ] {
        let key = format!("column.s3.object.{}", col.id());
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

pub fn console_url_buckets(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/s3/buckets?region={}",
        region, region
    )
}

pub fn console_url_bucket(region: &str, bucket: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        format!(
            "https://s3.console.aws.amazon.com/s3/buckets/{}?region={}",
            bucket, region
        )
    } else {
        format!(
            "https://s3.console.aws.amazon.com/s3/buckets/{}?region={}&prefix={}",
            bucket,
            region,
            urlencoding::encode(prefix)
        )
    }
}

#[derive(Debug, Clone)]
pub struct Bucket {
    pub name: String,
    pub region: String,
    pub creation_date: String,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub key: String,
    pub size: i64,
    pub last_modified: String,
    pub is_prefix: bool,
    pub storage_class: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BucketColumn {
    Name,
    Region,
    CreationDate,
}

impl BucketColumn {
    pub fn id(&self) -> &'static str {
        match self {
            BucketColumn::Name => "name",
            BucketColumn::Region => "region",
            BucketColumn::CreationDate => "creation_date",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            BucketColumn::Name => "Name",
            BucketColumn::Region => "Region",
            BucketColumn::CreationDate => "Creation date",
        }
    }

    pub fn all() -> Vec<ColumnId> {
        [
            BucketColumn::Name,
            BucketColumn::Region,
            BucketColumn::CreationDate,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "name" => Some(BucketColumn::Name),
            "region" => Some(BucketColumn::Region),
            "creation_date" => Some(BucketColumn::CreationDate),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.s3.bucket.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }
}

impl TableColumn<Bucket> for BucketColumn {
    fn name(&self) -> &str {
        let key = format!("column.s3.bucket.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.s3.bucket.{}", self.id()));
        translated.len().max(match self {
            BucketColumn::Name => 30,
            BucketColumn::Region => 15,
            BucketColumn::CreationDate => UTC_TIMESTAMP_WIDTH as usize,
        }) as u16
    }

    fn render(&self, item: &Bucket) -> (String, Style) {
        let text = match self {
            BucketColumn::Name => item.name.clone(),
            BucketColumn::Region => item.region.clone(),
            BucketColumn::CreationDate => item.creation_date.clone(),
        };
        (text, Style::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectColumn {
    Key,
    Type,
    LastModified,
    Size,
    StorageClass,
}

impl ObjectColumn {
    pub fn id(&self) -> &'static str {
        match self {
            ObjectColumn::Key => "key",
            ObjectColumn::Type => "type",
            ObjectColumn::LastModified => "last_modified",
            ObjectColumn::Size => "size",
            ObjectColumn::StorageClass => "storage_class",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            ObjectColumn::Key => "Name",
            ObjectColumn::Type => "Type",
            ObjectColumn::LastModified => "Last modified",
            ObjectColumn::Size => "Size",
            ObjectColumn::StorageClass => "Storage class",
        }
    }

    pub fn all() -> Vec<ColumnId> {
        [
            ObjectColumn::Key,
            ObjectColumn::Type,
            ObjectColumn::LastModified,
            ObjectColumn::Size,
            ObjectColumn::StorageClass,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "key" => Some(ObjectColumn::Key),
            "type" => Some(ObjectColumn::Type),
            "last_modified" => Some(ObjectColumn::LastModified),
            "size" => Some(ObjectColumn::Size),
            "storage_class" => Some(ObjectColumn::StorageClass),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.s3.object.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }
}

impl TableColumn<Object> for ObjectColumn {
    fn name(&self) -> &str {
        let key = format!("column.s3.object.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.s3.object.{}", self.id()));
        translated.len().max(match self {
            ObjectColumn::Key => 40,
            ObjectColumn::Type => 10,
            ObjectColumn::LastModified => UTC_TIMESTAMP_WIDTH as usize,
            ObjectColumn::Size => 15,
            ObjectColumn::StorageClass => 15,
        }) as u16
    }

    fn render(&self, item: &Object) -> (String, Style) {
        let text = match self {
            ObjectColumn::Key => {
                let icon = if item.is_prefix { "📁" } else { "📄" };
                format!("{} {}", icon, item.key)
            }
            ObjectColumn::Type => {
                if item.is_prefix {
                    "Folder".to_string()
                } else {
                    "File".to_string()
                }
            }
            ObjectColumn::LastModified => {
                if item.last_modified.is_empty() {
                    String::new()
                } else {
                    format!(
                        "{} (UTC)",
                        item.last_modified
                            .split('T')
                            .next()
                            .unwrap_or(&item.last_modified)
                    )
                }
            }
            ObjectColumn::Size => {
                if item.is_prefix {
                    String::new()
                } else {
                    format_bytes(item.size)
                }
            }
            ObjectColumn::StorageClass => {
                if item.storage_class.is_empty() {
                    String::new()
                } else {
                    item.storage_class
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .to_string()
                        + &item.storage_class[1..].to_lowercase()
                }
            }
        };
        (text, Style::default())
    }
}
