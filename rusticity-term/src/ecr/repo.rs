use crate::common::translate_column;
use crate::common::{format_iso_timestamp, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
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
    const ID_NAME: &'static str = "column.ecr.repo.name";
    const ID_URI: &'static str = "column.ecr.repo.uri";
    const ID_CREATED_AT: &'static str = "column.ecr.repo.created_at";
    const ID_TAG_IMMUTABILITY: &'static str = "column.ecr.repo.tag_immutability";
    const ID_ENCRYPTION_TYPE: &'static str = "column.ecr.repo.encryption_type";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Name => Self::ID_NAME,
            Column::Uri => Self::ID_URI,
            Column::CreatedAt => Self::ID_CREATED_AT,
            Column::TagImmutability => Self::ID_TAG_IMMUTABILITY,
            Column::EncryptionType => Self::ID_ENCRYPTION_TYPE,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Repository name",
            Column::Uri => "URI",
            Column::CreatedAt => "Created at",
            Column::TagImmutability => "Tag immutability",
            Column::EncryptionType => "Encryption type",
        }
    }

    pub const fn all() -> [Column; 5] {
        [
            Column::Name,
            Column::Uri,
            Column::CreatedAt,
            Column::TagImmutability,
            Column::EncryptionType,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_URI => Some(Column::Uri),
            Self::ID_CREATED_AT => Some(Column::CreatedAt),
            Self::ID_TAG_IMMUTABILITY => Some(Column::TagImmutability),
            Self::ID_ENCRYPTION_TYPE => Some(Column::EncryptionType),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<Repository> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.ecr.repo."),
                "Column ID '{}' should start with 'column.ecr.repo.'",
                col.id()
            );
        }
    }
}
