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
    const ID_TAG: &'static str = "column.ecr.image.tag";
    const ID_ARTIFACT_TYPE: &'static str = "column.ecr.image.artifact_type";
    const ID_PUSHED_AT: &'static str = "column.ecr.image.pushed_at";
    const ID_SIZE_MB: &'static str = "column.ecr.image.size_mb";
    const ID_URI: &'static str = "column.ecr.image.uri";
    const ID_DIGEST: &'static str = "column.ecr.image.digest";
    const ID_LAST_PULL_TIME: &'static str = "column.ecr.image.last_pull_time";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Tag => Self::ID_TAG,
            Column::ArtifactType => Self::ID_ARTIFACT_TYPE,
            Column::PushedAt => Self::ID_PUSHED_AT,
            Column::SizeMb => Self::ID_SIZE_MB,
            Column::Uri => Self::ID_URI,
            Column::Digest => Self::ID_DIGEST,
            Column::LastPullTime => Self::ID_LAST_PULL_TIME,
        }
    }

    pub const fn default_name(&self) -> &'static str {
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

    pub const fn all() -> [Column; 7] {
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
            Self::ID_TAG => Some(Column::Tag),
            Self::ID_ARTIFACT_TYPE => Some(Column::ArtifactType),
            Self::ID_PUSHED_AT => Some(Column::PushedAt),
            Self::ID_SIZE_MB => Some(Column::SizeMb),
            Self::ID_URI => Some(Column::Uri),
            Self::ID_DIGEST => Some(Column::Digest),
            Self::ID_LAST_PULL_TIME => Some(Column::LastPullTime),
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
