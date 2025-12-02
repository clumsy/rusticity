use crate::common::translate_column;
use crate::common::{format_iso_timestamp, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in [
        Column::Tag,
        Column::ArtifactType,
        Column::PushedAt,
        Column::SizeMb,
        Column::Uri,
        Column::Digest,
        Column::LastPullTime,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
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
            Column::Tag => "column.ecr.image.tag",
            Column::ArtifactType => "column.ecr.image.artifact_type",
            Column::PushedAt => "column.ecr.image.pushed_at",
            Column::SizeMb => "column.ecr.image.size_mb",
            Column::Uri => "column.ecr.image.uri",
            Column::Digest => "column.ecr.image.digest",
            Column::LastPullTime => "column.ecr.image.last_pull_time",
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

    pub fn all() -> [Column; 7] {
        [
            Column::Tag,
            Column::ArtifactType,
            Column::PushedAt,
            Column::SizeMb,
            Column::Uri,
            Column::Digest,
            Column::LastPullTime,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.ecr.image.tag" => Some(Column::Tag),
            "column.ecr.image.artifact_type" => Some(Column::ArtifactType),
            "column.ecr.image.pushed_at" => Some(Column::PushedAt),
            "column.ecr.image.size_mb" => Some(Column::SizeMb),
            "column.ecr.image.uri" => Some(Column::Uri),
            "column.ecr.image.digest" => Some(Column::Digest),
            "column.ecr.image.last_pull_time" => Some(Column::LastPullTime),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<Image> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.ecr.image."),
                "Column ID '{}' should start with 'column.ecr.image.'",
                col.id()
            );
        }
    }
}
