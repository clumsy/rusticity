use crate::common::translate_column;
use crate::common::{format_bytes, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in [
        BucketColumn::Name,
        BucketColumn::Region,
        BucketColumn::CreationDate,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }

    for col in [
        ObjectColumn::Key,
        ObjectColumn::Size,
        ObjectColumn::LastModified,
        ObjectColumn::StorageClass,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
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
            BucketColumn::Name => "column.s3.bucket.name",
            BucketColumn::Region => "column.s3.bucket.region",
            BucketColumn::CreationDate => "column.s3.bucket.creation_date",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            BucketColumn::Name => "Name",
            BucketColumn::Region => "Region",
            BucketColumn::CreationDate => "Creation date",
        }
    }

    pub fn all() -> [BucketColumn; 3] {
        [
            BucketColumn::Name,
            BucketColumn::Region,
            BucketColumn::CreationDate,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.s3.bucket.name" => Some(BucketColumn::Name),
            "column.s3.bucket.region" => Some(BucketColumn::Region),
            "column.s3.bucket.creation_date" => Some(BucketColumn::CreationDate),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<Bucket> for BucketColumn {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
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
            ObjectColumn::Key => "column.s3.object.key",
            ObjectColumn::Type => "column.s3.object.type",
            ObjectColumn::LastModified => "column.s3.object.last_modified",
            ObjectColumn::Size => "column.s3.object.size",
            ObjectColumn::StorageClass => "column.s3.object.storage_class",
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

    pub fn all() -> [ObjectColumn; 5] {
        [
            ObjectColumn::Key,
            ObjectColumn::Type,
            ObjectColumn::LastModified,
            ObjectColumn::Size,
            ObjectColumn::StorageClass,
        ]
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.s3.object.key" => Some(ObjectColumn::Key),
            "column.s3.object.type" => Some(ObjectColumn::Type),
            "column.s3.object.last_modified" => Some(ObjectColumn::LastModified),
            "column.s3.object.size" => Some(ObjectColumn::Size),
            "column.s3.object.storage_class" => Some(ObjectColumn::StorageClass),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<Object> for ObjectColumn {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_column_ids_have_correct_prefix() {
        for col in BucketColumn::all() {
            assert!(
                col.id().starts_with("column.s3.bucket."),
                "BucketColumn ID '{}' should start with 'column.s3.bucket.'",
                col.id()
            );
        }
    }

    #[test]
    fn test_object_column_ids_have_correct_prefix() {
        for col in ObjectColumn::all() {
            assert!(
                col.id().starts_with("column.s3.object."),
                "ObjectColumn ID '{}' should start with 'column.s3.object.'",
                col.id()
            );
        }
    }
}
