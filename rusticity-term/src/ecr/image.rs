use crate::common::t;
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
            Column::Tag => "tag",
            Column::ArtifactType => "artifact_type",
            Column::PushedAt => "pushed_at",
            Column::SizeMb => "size_mb",
            Column::Uri => "uri",
            Column::Digest => "digest",
            Column::LastPullTime => "last_pull_time",
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
            "tag" => Some(Column::Tag),
            "artifact_type" => Some(Column::ArtifactType),
            "pushed_at" => Some(Column::PushedAt),
            "size_mb" => Some(Column::SizeMb),
            "uri" => Some(Column::Uri),
            "digest" => Some(Column::Digest),
            "last_pull_time" => Some(Column::LastPullTime),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.ecr.image.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }
}

impl TableColumn<Image> for Column {
    fn name(&self) -> &str {
        let key = format!("column.ecr.image.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.ecr.image.{}", self.id()));
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
