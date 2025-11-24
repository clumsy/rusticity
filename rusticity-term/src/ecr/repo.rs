use crate::common::t;
use crate::common::{format_iso_timestamp, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in [
        Column::Name,
        Column::Uri,
        Column::CreatedAt,
        Column::TagImmutability,
        Column::EncryptionType,
    ] {
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

    pub fn all() -> [Column; 5] {
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
