use super::*;
use crate::app::ViewMode;
use crate::keymap::{Action, Mode};
use crate::preferences::PreferenceRow;
use rusticity_core::dynamodb::{DdbItem, DdbTable};

const SOURCES: &[Source] = &[
    Source::EcrRepositories,
    Source::EcrImages,
    Source::EfsFiles,
    Source::EfsTags,
    Source::EfsAccessPoints,
    Source::EfsMountTargets,
    Source::FsxFiles,
    Source::FsxTags,
    Source::FsxUpdates,
    Source::FsxBackups,
    Source::DdbTables,
    Source::DdbItems,
    Source::DdbIndexes,
    Source::DdbReplicas,
    Source::DdbBackups,
    Source::DdbTags,
    Source::KmsKeys,
    Source::RdsInstances,
    Source::VpcVpcs,
];

fn app_for(source: Source) -> App {
    let mut app = App::new_without_client("test".into(), None);
    app.service_selected = true;
    app.mode = Mode::ColumnSelector;
    // An unrelated legacy ViewMode must not override the service's own preferences.
    app.view_mode = ViewMode::Events;
    app.current_service = match source {
        Source::EcrRepositories | Source::EcrImages => Service::EcrRepositories,
        Source::EfsFiles | Source::EfsTags | Source::EfsAccessPoints | Source::EfsMountTargets => {
            Service::EfsFileSystems
        }
        Source::FsxFiles | Source::FsxTags | Source::FsxUpdates | Source::FsxBackups => {
            Service::FsxFileSystems
        }
        Source::KmsKeys => Service::KmsKeys,
        Source::RdsInstances => Service::RdsInstances,
        Source::VpcVpcs => Service::VpcVpcs,
        _ => Service::DynamoDbTables,
    };
    if matches!(source, Source::EcrImages) {
        app.ecr_state.current_repository = Some("repo".into());
    }
    match source {
        Source::EfsTags | Source::EfsAccessPoints | Source::EfsMountTargets => {
            app.efs_state.current_file_system = Some("fs".into());
            app.efs_state.detail_tab = match source {
                Source::EfsTags => efs::DetailTab::Tags,
                Source::EfsAccessPoints => efs::DetailTab::AccessPoints,
                _ => efs::DetailTab::Network,
            };
        }
        Source::FsxTags | Source::FsxUpdates | Source::FsxBackups => {
            app.fsx_state.current_file_system = Some("fs".into());
            app.fsx_state.detail_tab = match source {
                Source::FsxTags => fsx::DetailTab::Tags,
                Source::FsxUpdates => fsx::DetailTab::Updates,
                _ => fsx::DetailTab::Backups,
            };
        }
        Source::DdbItems
        | Source::DdbIndexes
        | Source::DdbReplicas
        | Source::DdbBackups
        | Source::DdbTags => {
            app.dynamodb_state.current_table = Some("orders".into());
            app.dynamodb_state.tables.items = vec![DdbTable {
                name: "orders".into(),
                partition_key: "pk (S)".into(),
                ..Default::default()
            }];
            let mut item = DdbItem::default();
            item.attributes.insert("status".into(), "ready".into());
            app.dynamodb_state.items.items = vec![item];
            app.dynamodb_state.detail_tab = match source {
                Source::DdbItems => dynamodb::DetailTab::Items,
                Source::DdbIndexes => dynamodb::DetailTab::Indexes,
                Source::DdbReplicas => dynamodb::DetailTab::GlobalTables,
                Source::DdbBackups => dynamodb::DetailTab::Backups,
                _ => dynamodb::DetailTab::Tags,
            };
        }
        _ => {}
    }
    app
}

#[test]
fn every_bound_panel_uses_model_navigation_and_page_size_actions() {
    for &source in SOURCES {
        let mut app = app_for(source);
        let model = active(&app).unwrap();
        assert_eq!(model.rows(), source.model(&app).rows());
        assert!(model.column_count() > 0);
        // Walk every row through the real App routing, skipping only separators.
        let expected: Vec<_> = (0..model.rows().len())
            .filter(|&i| !model.is_separator(i))
            .collect();
        for &index in expected.iter().skip(1) {
            app.handle_action(Action::NextItem);
            assert_eq!(app.column_selector_index, index);
        }
        app.handle_action(Action::NextItem);
        assert_eq!(app.column_selector_index, model.max_index());
        for &index in expected.iter().rev().skip(1) {
            app.handle_action(Action::PrevItem);
            assert_eq!(app.column_selector_index, index);
        }
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, model.move_section(0, true));
        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 0);

        for (index, row) in model.rows().iter().enumerate() {
            if let PreferenceRow::PageSize { value, .. } = row {
                app.column_selector_index = index;
                app.handle_action(Action::ToggleColumn);
                assert_eq!(source.page_size(&app), *value);
                assert!(matches!(
                    &active(&app).unwrap().rows()[index],
                    PreferenceRow::PageSize { selected: true, .. }
                ));
            }
        }
    }
}

#[test]
fn every_column_row_changes_only_its_named_column_or_is_locked() {
    for &source in SOURCES {
        let mut app = app_for(source);
        let before = active(&app).unwrap();
        for (index, row) in before.rows().iter().enumerate() {
            if let PreferenceRow::Column(column) = row {
                app.column_selector_index = index;
                app.handle_action(Action::ToggleColumn);
                let after = active(&app).unwrap();
                for (old, new) in before.columns().zip(after.columns()) {
                    if old.id == column.id && before.action(index).is_some() {
                        assert_eq!(new.visible, !old.visible);
                    } else {
                        assert_eq!(new, old);
                    }
                }
                app.handle_action(Action::ToggleColumn);
                assert_eq!(active(&app).unwrap().rows(), before.rows());
            }
        }
    }
}

#[test]
fn efs_tags_page_size_resets_only_tags_and_cannot_toggle_fixed_columns() {
    let mut app = app_for(Source::EfsTags);
    app.efs_state.tags_table.selected = 30;
    app.efs_state.tags_table.scroll_offset = 20;
    app.efs_state.tags_table.expanded_item = Some(30);
    app.efs_state.file_systems.selected = 17;
    let original = app.efs_visible_column_ids.clone();
    app.column_selector_index = 1;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.efs_visible_column_ids, original);
    app.column_selector_index = active(&app)
        .unwrap()
        .rows()
        .iter()
        .position(|r| {
            matches!(
                r,
                PreferenceRow::PageSize {
                    value: PageSize::Ten,
                    ..
                }
            )
        })
        .unwrap();
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.efs_state.tags_table.page_size, PageSize::Ten);
    assert_eq!(app.efs_state.tags_table.selected, 0);
    assert_eq!(app.efs_state.tags_table.scroll_offset, 0);
    assert_eq!(app.efs_state.tags_table.expanded_item, None);
    assert_eq!(app.efs_state.file_systems.selected, 17);
    assert_eq!(app.efs_visible_column_ids, original);
}

#[test]
fn kms_tabs_keep_separate_visibility_and_alias_is_locked() {
    let mut app = app_for(Source::KmsKeys);
    let aws_before = app.kms_state.aws_visible_column_ids.clone();
    app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;
    let customer_before = app.kms_state.customer_visible_column_ids.clone();
    app.column_selector_index = 1;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.kms_state.customer_visible_column_ids, customer_before);
    app.column_selector_index = 2;
    app.handle_action(Action::ToggleColumn);
    assert_ne!(app.kms_state.customer_visible_column_ids, customer_before);
    assert_eq!(app.kms_state.aws_visible_column_ids, aws_before);
}

#[test]
fn dynamic_columns_ignore_stale_hidden_ids_and_keep_last_visible() {
    let mut app = app_for(Source::DdbItems);
    app.dynamodb_state.item_hidden_columns = vec!["attribute-from-old-result".into()];
    let row_for = |app: &App, name: &str| {
        active(app)
            .unwrap()
            .rows()
            .iter()
            .position(|r| matches!(r, PreferenceRow::Column(c) if c.id == name))
            .unwrap()
    };
    app.column_selector_index = row_for(&app, "status");
    app.handle_action(Action::ToggleColumn);
    assert!(app
        .dynamodb_state
        .item_hidden_columns
        .iter()
        .any(|n| n == "status"));
    app.column_selector_index = row_for(&app, "pk");
    app.handle_action(Action::ToggleColumn);
    assert_eq!(
        dynamodb::item_visible_column_names(dynamodb::current_table(&app).unwrap(), &app),
        ["pk"]
    );
    // A hidden column can still be re-enabled despite unrelated stale names.
    app.column_selector_index = row_for(&app, "status");
    app.handle_action(Action::ToggleColumn);
    assert!(!app
        .dynamodb_state
        .item_hidden_columns
        .iter()
        .any(|n| n == "status"));

    // The renderer's all-hidden fallback and Preferences must agree.
    app.dynamodb_state.item_hidden_columns = vec!["pk".into(), "status".into()];
    assert!(active(&app).unwrap().columns().all(|c| c.visible));
    app.column_selector_index = row_for(&app, "status");
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.dynamodb_state.item_hidden_columns, ["status"]);
    assert_eq!(
        dynamodb::item_visible_column_names(dynamodb::current_table(&app).unwrap(), &app),
        ["pk"]
    );
}

#[test]
fn non_table_fallbacks_are_explicitly_read_only() {
    for source in [Source::EfsFiles, Source::FsxFiles, Source::DdbTables] {
        let mut app = app_for(source);
        match source {
            Source::EfsFiles => app.efs_state.current_file_system = Some("fs".into()),
            Source::FsxFiles => app.fsx_state.current_file_system = Some("fs".into()),
            _ => {
                app.dynamodb_state.current_table = Some("orders".into());
                app.dynamodb_state.detail_tab = dynamodb::DetailTab::Settings;
            }
        }
        let before = active(&app).unwrap();
        assert!(!before.is_enabled());
        for index in 0..before.rows().len() {
            assert_eq!(before.action(index), None);
            app.column_selector_index = index;
            app.handle_action(Action::ToggleColumn);
            assert_eq!(active(&app).unwrap().rows(), before.rows());
        }
    }
}

#[test]
fn stale_selector_index_is_safe_after_dynamic_schema_shrinks() {
    let mut app = app_for(Source::DdbItems);
    let original = app.dynamodb_state.item_hidden_columns.clone();
    app.column_selector_index = usize::MAX;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.dynamodb_state.item_hidden_columns, original);
    app.handle_action(Action::NextItem);
    assert_eq!(app.column_selector_index, active(&app).unwrap().max_index());
}
