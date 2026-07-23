use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::efs::EfsAccessPoint as AccessPoint;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    AccessPointId,
    Path,
    PosixUser,
    CreationInfo,
    State,
}

impl Column {
    const ID_NAME: &'static str = "column.efs.ap.name";
    const ID_ACCESS_POINT_ID: &'static str = "column.efs.ap.access_point_id";
    const ID_PATH: &'static str = "column.efs.ap.path";
    const ID_POSIX_USER: &'static str = "column.efs.ap.posix_user";
    const ID_CREATION_INFO: &'static str = "column.efs.ap.creation_info";
    const ID_STATE: &'static str = "column.efs.ap.state";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Name => Self::ID_NAME,
            Column::AccessPointId => Self::ID_ACCESS_POINT_ID,
            Column::Path => Self::ID_PATH,
            Column::PosixUser => Self::ID_POSIX_USER,
            Column::CreationInfo => Self::ID_CREATION_INFO,
            Column::State => Self::ID_STATE,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::AccessPointId => "Access point ID",
            Column::Path => "Path",
            Column::PosixUser => "POSIX user",
            Column::CreationInfo => "Creation info",
            Column::State => "State",
        }
    }

    pub const fn all() -> [Column; 6] {
        [
            Column::Name,
            Column::AccessPointId,
            Column::Path,
            Column::PosixUser,
            Column::CreationInfo,
            Column::State,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    /// All columns are visible by default.
    pub fn default_visible_ids() -> Vec<ColumnId> {
        Self::ids()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_ACCESS_POINT_ID => Some(Column::AccessPointId),
            Self::ID_PATH => Some(Column::Path),
            Self::ID_POSIX_USER => Some(Column::PosixUser),
            Self::ID_CREATION_INFO => Some(Column::CreationInfo),
            Self::ID_STATE => Some(Column::State),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<AccessPoint> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Name => 24,
            Column::AccessPointId => 22,
            Column::Path => 24,
            Column::PosixUser => 14,
            Column::CreationInfo => 22,
            Column::State => 14,
        }) as u16
    }

    fn render(&self, item: &AccessPoint) -> (String, Style) {
        match self {
            Column::Name => (item.name.clone(), Style::default()),
            Column::AccessPointId => (item.access_point_id.clone(), Style::default()),
            Column::Path => (item.path.clone(), Style::default()),
            Column::PosixUser => (item.posix_user.clone(), Style::default()),
            Column::CreationInfo => (item.creation_info.clone(), Style::default()),
            Column::State => {
                let (text, color) = format_state(&item.life_cycle_state);
                (text.to_string(), Style::default().fg(color))
            }
        }
    }
}

fn format_state(state: &str) -> (&'static str, Color) {
    match state {
        "available" | "Available" | "AVAILABLE" => ("✅ Available", Color::Green),
        "creating" | "Creating" | "CREATING" => ("⏳ Creating", Color::Yellow),
        "deleting" | "Deleting" | "DELETING" => ("🗑 Deleting", Color::Red),
        "deleted" | "Deleted" | "DELETED" => ("🗑 Deleted", Color::Red),
        "updating" | "Updating" | "UPDATING" => ("🔄 Updating", Color::Yellow),
        "error" | "Error" | "ERROR" => ("⚠ Error", Color::Red),
        _ => ("Unknown", Color::White),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ap_column_ids_have_prefix() {
        for col in Column::all() {
            assert!(col.id().starts_with("column.efs.ap."));
        }
    }

    #[test]
    fn test_ap_from_id_roundtrip() {
        for col in Column::all() {
            assert_eq!(Column::from_id(col.id()), Some(col));
        }
    }

    #[test]
    fn test_ap_all_six_columns_visible_by_default() {
        assert_eq!(Column::all().len(), 6);
        assert_eq!(Column::default_visible_ids().len(), 6);
    }
}
