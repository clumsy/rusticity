//! App-level tests for the shared sort chooser: the choice made in
//! Preferences orders the table, selection, copy and header alike.
use crate::app::{App, Service};
use crate::keymap::{Action, Mode};
use crate::preferences::{bindings, PreferenceRow};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::rds::RdsInstance;

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(240, 40)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// Activate the Preferences radio row `group`/`value`.
fn choose(app: &mut App, group: &str, value: &str) {
    let model = bindings::active(app).unwrap();
    let index = model
        .rows()
        .iter()
        .position(|row| {
            matches!(row, PreferenceRow::Radio { group: g, value: v, .. } if *g == group && *v == value)
        })
        .unwrap_or_else(|| panic!("no {group}={value} row"));
    app.mode = Mode::ColumnSelector;
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    app.mode = Mode::Normal;
}

fn selected_row(model: &crate::preferences::Preferences, group: &str) -> Option<&'static str> {
    model.rows().iter().find_map(|row| match row {
        PreferenceRow::Radio {
            group: g,
            value,
            selected: true,
            ..
        } if *g == group => Some(*value),
        _ => None,
    })
}

fn rds_app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.current_service = Service::RdsInstances;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.rds_state.instances.items = [("db-10", "postgres"), ("db-2", "mysql"), ("db-1", "aurora")]
        .into_iter()
        .map(|(id, engine)| RdsInstance {
            identifier: id.into(),
            arn: format!("arn:aws:rds:us-east-2:123456789012:db:{id}"),
            engine: engine.into(),
            ..Default::default()
        })
        .collect();
    crate::rds::inventory::mark_loaded(&mut app);
    app
}

fn rds_order(app: &App) -> Vec<String> {
    crate::ui::rds::filtered_instances(app)
        .iter()
        .map(|r| r.identifier.clone())
        .collect()
}

#[test]
fn the_default_order_is_natural_and_shown_in_preferences() {
    let app = rds_app();
    assert_eq!(rds_order(&app), ["db-1", "db-2", "db-10"]);
    let model = bindings::active(&app).unwrap();
    assert_eq!(
        selected_row(&model, crate::sort::COLUMN_GROUP),
        Some(crate::rds::instance::Column::Identifier.id())
    );
    assert_eq!(
        selected_row(&model, crate::sort::DIRECTION_GROUP),
        Some("asc")
    );
    assert!(screen(&app).contains("DB identifier ↑"));
}

#[test]
fn choosing_a_column_and_direction_reorders_rows_header_and_copy() {
    use crate::rds::instance::Column;
    let mut app = rds_app();
    app.rds_state.instances.selected = 2;
    choose(&mut app, crate::sort::COLUMN_GROUP, Column::Engine.id());
    assert_eq!(
        rds_order(&app),
        ["db-1", "db-2", "db-10"],
        "aurora, mysql, postgres"
    );
    assert_eq!(
        app.rds_state.instances.selected, 0,
        "the new order starts at the top"
    );
    choose(&mut app, crate::sort::DIRECTION_GROUP, "desc");
    assert_eq!(rds_order(&app), ["db-10", "db-2", "db-1"]);
    let text = screen(&app);
    assert!(text.contains("Engine ↓") && !text.contains("DB identifier ↑"));
    // Copy follows the order on screen.
    assert_eq!(
        crate::rds::actions::selected_arn(&app).as_deref(),
        Some("arn:aws:rds:us-east-2:123456789012:db:db-10")
    );
}

#[test]
fn hiding_the_sort_column_falls_back_to_the_default_order() {
    use crate::rds::instance::Column;
    let mut app = rds_app();
    choose(&mut app, crate::sort::COLUMN_GROUP, Column::Engine.id());
    choose(&mut app, crate::sort::DIRECTION_GROUP, "desc");
    app.rds_state
        .visible_column_ids
        .retain(|id| *id != Column::Engine.id());
    assert_eq!(rds_order(&app), ["db-1", "db-2", "db-10"]);
    let model = bindings::active(&app).unwrap();
    assert_eq!(
        selected_row(&model, crate::sort::COLUMN_GROUP),
        Some(Column::Identifier.id())
    );
    assert!(!model.rows().iter().any(|row| matches!(
        row,
        PreferenceRow::Radio { value, .. } if *value == Column::Engine.id()
    )));
}

#[test]
fn every_chooser_table_offers_sort_rows_and_applies_them() {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.service_selected = true;
    for service in [
        Service::RdsInstances,
        Service::VpcVpcs,
        Service::SecretsManagerSecrets,
        Service::AcmCertificates,
        Service::Ec2Instances,
    ] {
        app.current_service = service;
        app.mode = Mode::Normal;
        let model = bindings::model_for(&app, service);
        assert!(
            selected_row(&model, crate::sort::COLUMN_GROUP).is_some(),
            "{service:?} sort by"
        );
        choose(&mut app, crate::sort::DIRECTION_GROUP, "desc");
        let model = bindings::model_for(&app, service);
        assert_eq!(
            selected_row(&model, crate::sort::DIRECTION_GROUP),
            Some("desc"),
            "{service:?} direction"
        );
    }
}

#[test]
fn ec2_selection_and_screen_use_one_list() {
    use crate::app::Ec2Instance;
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.current_service = Service::Ec2Instances;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.ec2_state.table.items = ["i-0b", "i-0a", "i-0c"]
        .into_iter()
        .map(|id| Ec2Instance {
            instance_id: id.into(),
            name: format!("{id}-name"),
            state: "running".into(),
            ..Default::default()
        })
        .collect();
    choose(
        &mut app,
        crate::sort::COLUMN_GROUP,
        crate::ec2::Column::InstanceId.id(),
    );
    // EC2 defaults to newest launch first, so the direction is still Desc.
    choose(&mut app, crate::sort::DIRECTION_GROUP, "asc");
    let ids: Vec<_> = crate::ui::ec2::filtered_ec2_instances(&app)
        .iter()
        .map(|i| i.instance_id.clone())
        .collect();
    assert_eq!(ids, ["i-0a", "i-0b", "i-0c"]);
    // Filtering by key pair name, which only the screen used to honour.
    app.ec2_state.table.items[0].key_name = "deploy".into();
    app.ec2_state.table.filter = "deploy".into();
    let ids: Vec<_> = crate::ui::ec2::filtered_ec2_instances(&app)
        .iter()
        .map(|i| i.instance_id.clone())
        .collect();
    assert_eq!(ids, ["i-0b"]);
}
