//! Bind the shared row model to service state. No row-index arithmetic lives here.
use super::{PreferenceAction, PreferenceColumn, Preferences};
use crate::app::{App, Service};
use crate::common::{ColumnId, PageSize};
use crate::ui::{dynamodb, efs, fsx};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Source {
    EcrRepositories,
    EcrImages,
    EfsFiles,
    EfsTags,
    EfsAccessPoints,
    EfsMountTargets,
    FsxFiles,
    FsxTags,
    FsxUpdates,
    FsxBackups,
    DdbTables,
    DdbItems,
    DdbIndexes,
    DdbReplicas,
    DdbBackups,
    DdbTags,
    KmsKeys,
}

fn columns(
    ids: &[ColumnId],
    visible: &[ColumnId],
    name: impl Fn(&str) -> Option<String>,
    locked: Option<ColumnId>,
) -> Vec<PreferenceColumn> {
    ids.iter()
        .filter_map(|&id| {
            name(id).map(|label| PreferenceColumn {
                id: Cow::Borrowed(id),
                label,
                visible: visible.contains(&id),
                locked: locked == Some(id),
            })
        })
        .collect()
}

fn fixed(names: &[&'static str]) -> Vec<PreferenceColumn> {
    names
        .iter()
        .map(|&label| PreferenceColumn {
            id: Cow::Borrowed(label),
            label: label.into(),
            visible: true,
            locked: true,
        })
        .collect()
}

impl Source {
    fn for_service(app: &App, service: Service) -> Option<Self> {
        Some(match service {
            Service::EcrRepositories => {
                if app.ecr_state.current_repository.is_some() {
                    Self::EcrImages
                } else {
                    Self::EcrRepositories
                }
            }
            Service::EfsFileSystems => {
                if app.efs_state.current_file_system.is_some() {
                    match app.efs_state.detail_tab {
                        efs::DetailTab::Tags => Self::EfsTags,
                        efs::DetailTab::AccessPoints => Self::EfsAccessPoints,
                        efs::DetailTab::Network => Self::EfsMountTargets,
                        _ => Self::EfsFiles,
                    }
                } else {
                    Self::EfsFiles
                }
            }
            Service::FsxFileSystems => {
                if app.fsx_state.current_file_system.is_some() {
                    match app.fsx_state.detail_tab {
                        fsx::DetailTab::Tags => Self::FsxTags,
                        fsx::DetailTab::Updates => Self::FsxUpdates,
                        fsx::DetailTab::Backups => Self::FsxBackups,
                        _ => Self::FsxFiles,
                    }
                } else {
                    Self::FsxFiles
                }
            }
            Service::DynamoDbTables => {
                if app.dynamodb_state.current_table.is_some() {
                    match app.dynamodb_state.detail_tab {
                        dynamodb::DetailTab::Items => Self::DdbItems,
                        dynamodb::DetailTab::Indexes => Self::DdbIndexes,
                        dynamodb::DetailTab::GlobalTables => Self::DdbReplicas,
                        dynamodb::DetailTab::Backups => Self::DdbBackups,
                        dynamodb::DetailTab::Tags => Self::DdbTags,
                        _ => Self::DdbTables,
                    }
                } else {
                    Self::DdbTables
                }
            }
            Service::KmsKeys => Self::KmsKeys,
            _ => return None,
        })
    }

    fn model(self, app: &App) -> Preferences {
        let cols = match self {
            Self::EcrRepositories => columns(
                &app.ecr_repo_column_ids,
                &app.ecr_repo_visible_column_ids,
                |id| crate::ecr::repo::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::EcrImages => columns(
                &app.ecr_image_column_ids,
                &app.ecr_image_visible_column_ids,
                |id| crate::ecr::image::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::EfsFiles => columns(
                &app.efs_column_ids,
                &app.efs_visible_column_ids,
                |id| crate::efs::fs::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::EfsAccessPoints => columns(
                &app.efs_state.ap_column_ids,
                &app.efs_state.ap_visible_column_ids,
                |id| crate::efs::access_point::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::EfsMountTargets => columns(
                &app.efs_state.mt_column_ids,
                &app.efs_state.mt_visible_column_ids,
                |id| crate::efs::mount_target::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::EfsTags => fixed(&["Key", "Value"]),
            Self::FsxFiles => columns(
                &app.fsx_column_ids,
                &app.fsx_visible_column_ids,
                |id| crate::fsx::fs::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::FsxTags => fixed(&fsx::TAGS_COLUMNS),
            Self::FsxUpdates => fixed(&fsx::UPDATES_COLUMNS),
            Self::FsxBackups => fixed(&fsx::BACKUPS_COLUMNS),
            Self::DdbTables => columns(
                &app.dynamodb_column_ids,
                &app.dynamodb_visible_column_ids,
                |id| crate::dynamodb::table::Column::from_id(id).map(|c| c.name()),
                None,
            ),
            Self::DdbItems => dynamodb::current_table(app)
                .map(|table| {
                    let visible = dynamodb::item_visible_column_names(table, app);
                    dynamodb::item_column_names(table, app)
                        .into_iter()
                        .map(|name| PreferenceColumn {
                            visible: visible.contains(&name),
                            label: name.clone(),
                            id: Cow::Owned(name),
                            locked: false,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Self::DdbIndexes => fixed(&dynamodb::INDEXES_COLUMNS),
            Self::DdbReplicas => fixed(&dynamodb::REPLICAS_COLUMNS),
            Self::DdbBackups => fixed(&dynamodb::BACKUPS_COLUMNS),
            Self::DdbTags => fixed(&dynamodb::TAGS_COLUMNS),
            Self::KmsKeys => columns(
                &app.kms_state.tab_column_ids(),
                app.kms_state.visible_column_ids(),
                |id| crate::kms::key::Column::from_id(id).map(|c| c.name()),
                Some(crate::kms::key::Column::Alias.id()),
            ),
        };
        let prefs = Preferences::table(cols, self.page_size(app));
        // Existing non-table panes show the parent schema, but cannot mutate it.
        let read_only = match self {
            Self::EfsFiles => app.efs_state.current_file_system.is_some(),
            Self::FsxFiles => app.fsx_state.current_file_system.is_some(),
            Self::DdbTables => app.dynamodb_state.current_table.is_some(),
            _ => false,
        };
        if read_only {
            prefs.read_only()
        } else {
            prefs
        }
    }

    fn page_size(self, app: &App) -> PageSize {
        match self {
            Self::EcrRepositories => app.ecr_state.repositories.page_size,
            Self::EcrImages => app.ecr_state.images.page_size,
            Self::EfsFiles => app.efs_state.file_systems.page_size,
            Self::EfsTags => app.efs_state.tags_table.page_size,
            Self::EfsAccessPoints => app.efs_state.access_points.page_size,
            Self::EfsMountTargets => app.efs_state.mount_targets.page_size,
            Self::FsxFiles => app.fsx_state.file_systems.page_size,
            Self::FsxTags => app.fsx_state.tags_table.page_size,
            Self::FsxUpdates => app.fsx_state.updates_table.page_size,
            Self::FsxBackups => app.fsx_state.backups.page_size,
            Self::DdbTables => app.dynamodb_state.tables.page_size,
            Self::DdbItems => app.dynamodb_state.items.page_size,
            Self::DdbIndexes => app.dynamodb_state.indexes_table.page_size,
            Self::DdbReplicas => app.dynamodb_state.replicas_table.page_size,
            Self::DdbBackups => app.dynamodb_state.backups.page_size,
            Self::DdbTags => app.dynamodb_state.tags_table.page_size,
            Self::KmsKeys => app.kms_state.keys.page_size,
        }
    }

    fn set_page_size(self, app: &mut App, size: PageSize) {
        match self {
            Self::EcrRepositories => app.ecr_state.repositories.set_page_size(size),
            Self::EcrImages => app.ecr_state.images.set_page_size(size),
            Self::EfsFiles => app.efs_state.file_systems.set_page_size(size),
            Self::EfsTags => app.efs_state.tags_table.set_page_size(size),
            Self::EfsAccessPoints => app.efs_state.access_points.set_page_size(size),
            Self::EfsMountTargets => app.efs_state.mount_targets.set_page_size(size),
            Self::FsxFiles => app.fsx_state.file_systems.set_page_size(size),
            Self::FsxTags => app.fsx_state.tags_table.set_page_size(size),
            Self::FsxUpdates => app.fsx_state.updates_table.set_page_size(size),
            Self::FsxBackups => app.fsx_state.backups.set_page_size(size),
            Self::DdbTables => app.dynamodb_state.tables.set_page_size(size),
            Self::DdbItems => app.dynamodb_state.items.set_page_size(size),
            Self::DdbIndexes => app.dynamodb_state.indexes_table.set_page_size(size),
            Self::DdbReplicas => app.dynamodb_state.replicas_table.set_page_size(size),
            Self::DdbBackups => app.dynamodb_state.backups.set_page_size(size),
            Self::DdbTags => app.dynamodb_state.tags_table.set_page_size(size),
            Self::KmsKeys => app.kms_state.keys.set_page_size(size),
        }
    }

    fn visible_columns(self, app: &mut App) -> Option<&mut Vec<ColumnId>> {
        Some(match self {
            Self::EcrRepositories => &mut app.ecr_repo_visible_column_ids,
            Self::EcrImages => &mut app.ecr_image_visible_column_ids,
            Self::EfsFiles => &mut app.efs_visible_column_ids,
            Self::EfsAccessPoints => &mut app.efs_state.ap_visible_column_ids,
            Self::EfsMountTargets => &mut app.efs_state.mt_visible_column_ids,
            Self::FsxFiles => &mut app.fsx_visible_column_ids,
            Self::DdbTables => &mut app.dynamodb_visible_column_ids,
            Self::KmsKeys => app.kms_state.visible_column_ids_mut(),
            _ => return None,
        })
    }
}

pub(crate) fn active(app: &App) -> Option<Preferences> {
    Source::for_service(app, app.current_service).map(|source| source.model(app))
}

/// Explicit service routing also supports legacy service entry points used by callers/tests.
pub(crate) fn model_for(app: &App, service: Service) -> Preferences {
    Source::for_service(app, service)
        .expect("service has a preferences model")
        .model(app)
}

pub(crate) fn activate_active(app: &mut App) -> bool {
    let service = app.current_service;
    activate_for(app, service)
}

pub(crate) fn activate_for(app: &mut App, service: Service) -> bool {
    let Some(source) = Source::for_service(app, service) else {
        return false;
    };
    let model = source.model(app);
    match model.action(app.column_selector_index) {
        Some(PreferenceAction::SetPageSize(size)) => source.set_page_size(app, size),
        Some(PreferenceAction::SetColumnVisibility { id, visible }) => {
            if matches!(source, Source::DdbItems) {
                let hidden = &mut app.dynamodb_state.item_hidden_columns;
                // The renderer falls back to all visible if a changed result set
                // contains only hidden attributes. Normalize those effective rows
                // before applying this action, but retain unrelated preferences.
                hidden.retain(|name| !model.columns().any(|c| c.visible && c.id.as_ref() == name));
                hidden.retain(|name| name != id.as_ref());
                if !visible {
                    hidden.push(id.into_owned());
                }
            } else if let (Some(columns), Cow::Borrowed(id)) = (source.visible_columns(app), id) {
                columns.retain(|column| *column != id);
                if visible {
                    columns.push(id);
                }
            }
        }
        None => {}
    }
    true
}

pub(crate) fn move_section_active(app: &mut App, forward: bool) -> bool {
    let service = app.current_service;
    move_section_for(app, service, forward)
}

pub(crate) fn move_section_for(app: &mut App, service: Service, forward: bool) -> bool {
    let Some(source) = Source::for_service(app, service) else {
        return false;
    };
    app.column_selector_index = source
        .model(app)
        .move_section(app.column_selector_index, forward);
    true
}

#[cfg(test)]
mod tests;
