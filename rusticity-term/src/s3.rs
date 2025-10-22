use crate::common::{format_bytes, ColumnTrait, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;

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
    pub fn name(&self) -> &'static str {
        match self {
            BucketColumn::Name => "Name",
            BucketColumn::Region => "Region",
            BucketColumn::CreationDate => "Creation date",
        }
    }

    pub fn all() -> Vec<BucketColumn> {
        vec![
            BucketColumn::Name,
            BucketColumn::Region,
            BucketColumn::CreationDate,
        ]
    }
}

impl TableColumn<Bucket> for BucketColumn {
    fn name(&self) -> &str {
        BucketColumn::name(self)
    }

    fn width(&self) -> u16 {
        self.name().len().max(match self {
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

impl ColumnTrait for BucketColumn {
    fn name(&self) -> &'static str {
        BucketColumn::name(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectColumn {
    Name,
    Type,
    LastModified,
    Size,
    StorageClass,
}

impl ObjectColumn {
    pub fn name(&self) -> &'static str {
        match self {
            ObjectColumn::Name => "Name",
            ObjectColumn::Type => "Type",
            ObjectColumn::LastModified => "Last modified",
            ObjectColumn::Size => "Size",
            ObjectColumn::StorageClass => "Storage class",
        }
    }

    pub fn all() -> Vec<ObjectColumn> {
        vec![
            ObjectColumn::Name,
            ObjectColumn::Type,
            ObjectColumn::LastModified,
            ObjectColumn::Size,
            ObjectColumn::StorageClass,
        ]
    }
}

impl TableColumn<Object> for ObjectColumn {
    fn name(&self) -> &str {
        ObjectColumn::name(self)
    }

    fn width(&self) -> u16 {
        self.name().len().max(match self {
            ObjectColumn::Name => 40,
            ObjectColumn::Type => 10,
            ObjectColumn::LastModified => UTC_TIMESTAMP_WIDTH as usize,
            ObjectColumn::Size => 15,
            ObjectColumn::StorageClass => 15,
        }) as u16
    }

    fn render(&self, item: &Object) -> (String, Style) {
        let text = match self {
            ObjectColumn::Name => {
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

impl ColumnTrait for ObjectColumn {
    fn name(&self) -> &'static str {
        ObjectColumn::name(self)
    }
}
