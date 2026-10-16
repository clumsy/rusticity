use crate::{
    common::{format_iso_timestamp, translate_column, ColumnId},
    ui::table::Column as TableColumn,
};
use ratatui::prelude::*;
use rusticity_core::SecretMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Description,
    LastAccessed,
    Created,
    Deleted,
    LastChanged,
    LastRotated,
    NextRotation,
    ReplicationType,
    ManagedBy,
    Type,
}
impl Column {
    pub const ALL: [Self; 11] = [
        Self::Name,
        Self::Description,
        Self::LastAccessed,
        Self::Created,
        Self::Deleted,
        Self::LastChanged,
        Self::LastRotated,
        Self::NextRotation,
        Self::ReplicationType,
        Self::ManagedBy,
        Self::Type,
    ];
    fn definition(self) -> (ColumnId, &'static str, u16) {
        match self {
            Self::Name => ("column.secretsmanager.secret.name", "Secret name", 32),
            Self::Description => (
                "column.secretsmanager.secret.description",
                "Description",
                48,
            ),
            Self::LastAccessed => (
                "column.secretsmanager.secret.last_accessed",
                "Last retrieved (UTC)",
                22,
            ),
            Self::Created => (
                "column.secretsmanager.secret.created",
                "Created on (UTC)",
                27,
            ),
            Self::Deleted => (
                "column.secretsmanager.secret.deleted",
                "Deleted on (UTC)",
                27,
            ),
            Self::LastChanged => (
                "column.secretsmanager.secret.last_changed",
                "Last updated on (UTC)",
                27,
            ),
            Self::LastRotated => (
                "column.secretsmanager.secret.last_rotated",
                "Last rotated date (UTC)",
                27,
            ),
            Self::NextRotation => (
                "column.secretsmanager.secret.next_rotation",
                "Next rotation date (UTC)",
                27,
            ),
            Self::ReplicationType => (
                "column.secretsmanager.secret.replication_type",
                "Replication type",
                18,
            ),
            Self::ManagedBy => ("column.secretsmanager.secret.managed_by", "Managed by", 24),
            Self::Type => ("column.secretsmanager.secret.type", "Type", 25),
        }
    }
    pub fn id(self) -> ColumnId {
        self.definition().0
    }
    pub fn default_name(self) -> &'static str {
        self.definition().1
    }
    pub fn name(self) -> String {
        translate_column(self.id(), self.default_name())
    }
    pub fn ids() -> Vec<ColumnId> {
        Self::ALL.into_iter().map(Self::id).collect()
    }
    pub fn default_visible_ids() -> Vec<ColumnId> {
        [
            Self::Name,
            Self::Description,
            Self::LastAccessed,
            Self::LastChanged,
            Self::ManagedBy,
        ]
        .into_iter()
        .map(Self::id)
        .collect()
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
    pub fn render(self, secret: &SecretMetadata) -> (String, Style) {
        let text = match self {
            Self::Name => secret.name.clone(),
            Self::Description => secret.description.clone(),
            Self::LastAccessed => secret.last_accessed_date.clone(),
            Self::Created => format_iso_timestamp(&secret.created_date),
            Self::Deleted => format_iso_timestamp(&secret.deleted_date),
            Self::LastChanged => format_iso_timestamp(&secret.last_changed_date),
            Self::LastRotated => format_iso_timestamp(&secret.last_rotated_date),
            Self::NextRotation => format_iso_timestamp(&secret.next_rotation_date),
            Self::ManagedBy => secret.owning_service.clone(),
            Self::Type => secret.secret_type.clone(),
            Self::ReplicationType => replication_type(secret).into(),
        };
        let text = if text.is_empty() {
            "-".into()
        } else {
            text.chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect()
        };
        (text, Style::default())
    }
    pub fn into_table_column(self) -> Box<dyn TableColumn<SecretMetadata>> {
        Box::new(crate::ui::table::OwnedColumn::new(
            self.id(),
            self.default_name(),
            self.definition().2,
            self != Self::Description,
            move |secret: &SecretMetadata| self.render(secret),
        ))
    }
}
/// Classify only when AWS supplies PrimaryRegion and the resource Region.
/// Absence is not evidence of a replica or a guessed replication state.
pub fn replication_type(secret: &SecretMetadata) -> &'static str {
    if secret.primary_region.is_empty() {
        return "-";
    }
    match secret.arn.split(':').nth(3).filter(|r| !r.is_empty()) {
        Some(region) if region == secret.primary_region => "Primary",
        Some(_) => "Replica",
        None => "-",
    }
}
