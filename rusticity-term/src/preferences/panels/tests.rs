use super::*;
use crate::apig::api::RestApi;
use crate::keymap::{Action, Mode};
use crate::preferences::{bindings, PreferenceRow};

const PANELS: &[Panel] = &[
    Panel::Buckets,
    Panel::Alarms,
    Panel::Trail,
    Panel::TrailResources,
    Panel::LogGroups,
    Panel::LogStreams,
    Panel::LogEvents,
    Panel::LogTags,
    Panel::Apis,
    Panel::ApiRoutes,
    Panel::ApiResources,
    Panel::Ec2,
    Panel::Queues,
    Panel::Triggers,
    Panel::Subscriptions,
    Panel::Pipes,
    Panel::QueueTags,
    Panel::Functions,
    Panel::Versions,
    Panel::Aliases,
    Panel::Layers,
    Panel::Applications,
    Panel::AppResources,
    Panel::Deployments,
    Panel::Stacks,
    Panel::Parameters,
    Panel::Outputs,
    Panel::Resources,
    Panel::StackEvents,
    Panel::Changes,
    Panel::Users,
    Panel::Roles,
    Panel::Groups,
    Panel::Policies,
    Panel::StackTags,
    Panel::Memberships,
    Panel::LastAccessed,
    Panel::Empty,
];

fn app() -> App {
    let mut app = App::new_without_client("test".into(), None);
    app.mode = Mode::ColumnSelector;
    app.service_selected = true;
    app
}

fn activate(app: &mut App, action: PreferenceAction) {
    let model = bindings::model_for(app, app.current_service);
    app.column_selector_index = (0..model.rows().len())
        .find(|&i| model.action(i) == Some(action.clone()))
        .unwrap();
    app.handle_action(Action::ToggleColumn);
}

#[test]
fn every_panel_uses_its_rendered_rows_for_activation() {
    for &panel in PANELS {
        let mut app = app();
        let model = panel.model(&app);
        assert_eq!(model.max_index(), model.rows().len().saturating_sub(1));
        for index in 0..model.rows().len() {
            let before = panel.model(&app);
            let action = before.action(index);
            app.column_selector_index = index;
            panel.activate(&mut app);
            let after = panel.model(&app);
            match action {
                Some(PreferenceAction::SetColumnVisibility { id, visible }) => {
                    assert_eq!(
                        after
                            .columns()
                            .find(|column| column.id == id)
                            .unwrap()
                            .visible,
                        visible
                    );
                    panel.activate(&mut app);
                    assert_eq!(panel.model(&app).rows(), before.rows());
                }
                Some(PreferenceAction::SetPageSize(size)) => {
                    assert_eq!(panel.page_size(&app), Some(size))
                }
                None => assert_eq!(after.rows(), before.rows()),
                _ => {}
            }
        }
        let before = panel.model(&app);
        app.column_selector_index = usize::MAX;
        panel.activate(&mut app);
        assert_eq!(panel.model(&app).rows(), before.rows());
        assert_eq!(before.action(usize::MAX), None);
        if panel.small_pages() {
            assert!(!before.rows().iter().any(|row| matches!(
                row,
                PreferenceRow::PageSize {
                    value: PageSize::OneHundred,
                    ..
                }
            )));
        }
    }
}

#[test]
fn alarms_radio_toggle_and_section_navigation_share_the_schema() {
    let mut app = app();
    app.current_service = Service::CloudWatchAlarms;
    let model = bindings::model_for(&app, app.current_service);
    let sections: Vec<_> = model
        .rows()
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            if let PreferenceRow::Section(label) = row {
                Some((i, *label))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        sections.iter().map(|(_, label)| *label).collect::<Vec<_>>(),
        ["Columns", "View as", "Page size", "Wrap lines"]
    );
    for &(index, _) in sections.iter().skip(1).chain(sections.iter().take(1)) {
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, index);
    }
    app.handle_action(Action::PrevPreferences);
    assert_eq!(app.column_selector_index, sections.last().unwrap().0);
    activate(
        &mut app,
        PreferenceAction::SetRadio {
            group: "alarm-view",
            value: "cards",
        },
    );
    assert_eq!(app.alarms_state.view_as, AlarmViewMode::Cards);
    activate(
        &mut app,
        PreferenceAction::SetRadio {
            group: "alarm-view",
            value: "table",
        },
    );
    assert_eq!(app.alarms_state.view_as, AlarmViewMode::Table);
    let wrapped = app.alarms_state.wrap_lines;
    activate(
        &mut app,
        PreferenceAction::SetToggle {
            id: "alarm-wrap",
            enabled: !wrapped,
        },
    );
    assert_eq!(app.alarms_state.wrap_lines, !wrapped);
    activate(
        &mut app,
        PreferenceAction::SetToggle {
            id: "alarm-wrap",
            enabled: wrapped,
        },
    );
    assert_eq!(app.alarms_state.wrap_lines, wrapped);
    app.alarms_state.table.selected = 28;
    app.alarms_state.table.expanded_item = Some(28);
    activate(&mut app, PreferenceAction::SetPageSize(PageSize::Ten));
    assert_eq!(app.alarms_state.table.selected, 28);
    assert_eq!(app.alarms_state.table.expanded_item, Some(28));
}

#[test]
fn version_alias_preferences_use_alias_page_size_without_touching_parent_tables() {
    let mut app = app();
    app.current_service = Service::LambdaFunctions;
    app.lambda_state.current_version = Some("1".into());
    app.lambda_state.detail_tab = lambda::DetailTab::Configuration;
    app.lambda_state.table.page_size = PageSize::OneHundred;
    app.lambda_state.alias_table.page_size = PageSize::TwentyFive;
    assert!(matches!(
        Panel::for_service(&app, app.current_service),
        Panel::Aliases
    ));
    activate(&mut app, PreferenceAction::SetPageSize(PageSize::Ten));
    assert_eq!(app.lambda_state.alias_table.page_size, PageSize::Ten);
    assert_eq!(app.lambda_state.table.page_size, PageSize::OneHundred);
    assert_eq!(app.lambda_state.version_table.page_size, PageSize::Fifty);
    assert!(!Panel::Layers
        .model(&app)
        .rows()
        .iter()
        .any(|row| matches!(row, PreferenceRow::PageSize { .. })));
}

#[test]
fn tree_preferences_lock_the_path_and_do_not_offer_fake_pagination() {
    let mut app = app();
    app.current_service = Service::ApiGatewayApis;
    app.apig_state.detail_tab = apig::ApiDetailTab::Routes;
    for protocol in ["REST", "HTTP"] {
        app.apig_state.current_api = Some(RestApi {
            id: "api".into(),
            name: "api".into(),
            protocol_type: protocol.into(),
            description: String::new(),
            created_date: String::new(),
            api_key_source: String::new(),
            endpoint_configuration: String::new(),
            disable_execute_api_endpoint: false,
            status: String::new(),
        });
        let model = bindings::model_for(&app, app.current_service);
        assert_eq!(model.rows().len(), model.column_count() + 1);
        assert!(model.columns().next().unwrap().locked);
        assert_eq!(model.action(1), None);
    }
}

#[test]
fn logs_events_are_headerless_and_streams_do_not_fabricate_a_selected_page_size() {
    let mut app = app();
    app.current_service = Service::CloudWatchLogGroups;
    app.view_mode = ViewMode::Events;
    let model = bindings::model_for(&app, app.current_service);
    assert_eq!(model.rows().len(), app.cw_log_event_column_ids.len());
    assert!(matches!(model.rows()[0], PreferenceRow::Column(_)));
    assert!(matches!(
        model.action(0),
        Some(PreferenceAction::SetColumnVisibility { .. })
    ));
    app.view_mode = ViewMode::Detail;
    app.log_groups_state.stream_page_size = 20;
    let model = bindings::model_for(&app, app.current_service);
    assert!(!model
        .rows()
        .iter()
        .any(|row| matches!(row, PreferenceRow::PageSize { selected: true, .. })));
    activate(
        &mut app,
        PreferenceAction::SetPageSize(PageSize::TwentyFive),
    );
    assert_eq!(app.log_groups_state.stream_page_size, 25);
    assert_eq!(app.log_groups_state.log_groups.page_size, PageSize::Fifty);
}

#[test]
fn cloudtrail_keeps_the_selected_event_when_page_size_changes() {
    let mut app = app();
    app.current_service = Service::CloudTrailEvents;
    app.cloudtrail_state.table.selected = 28;
    activate(
        &mut app,
        PreferenceAction::SetPageSize(PageSize::TwentyFive),
    );
    assert_eq!(app.cloudtrail_state.table.selected, 28);
    assert_eq!(app.cloudtrail_state.table.scroll_offset, 25);
}
