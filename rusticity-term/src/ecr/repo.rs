use crate::common::{format_iso_timestamp, ColumnTrait, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;

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
    pub fn all() -> Vec<Column> {
        vec![
            Column::Name,
            Column::Uri,
            Column::CreatedAt,
            Column::TagImmutability,
            Column::EncryptionType,
        ]
    }
}

impl ColumnTrait for Column {
    fn name(&self) -> &'static str {
        match self {
            Column::Name => "Repository name",
            Column::Uri => "URI",
            Column::CreatedAt => "Created at",
            Column::TagImmutability => "Tag immutability",
            Column::EncryptionType => "Encryption type",
        }
    }
}

impl TableColumn<Repository> for Column {
    fn name(&self) -> &str {
        ColumnTrait::name(self)
    }

    fn width(&self) -> u16 {
        ColumnTrait::name(self).len().max(match self {
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
