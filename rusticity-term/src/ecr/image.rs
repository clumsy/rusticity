use crate::common::{format_iso_timestamp, ColumnTrait, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;

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
    pub fn all() -> Vec<Column> {
        vec![
            Column::Tag,
            Column::ArtifactType,
            Column::PushedAt,
            Column::SizeMb,
            Column::Uri,
            Column::Digest,
            Column::LastPullTime,
        ]
    }
}

impl ColumnTrait for Column {
    fn name(&self) -> &'static str {
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
}

impl TableColumn<Image> for Column {
    fn name(&self) -> &str {
        ColumnTrait::name(self)
    }

    fn width(&self) -> u16 {
        ColumnTrait::name(self).len().max(match self {
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
