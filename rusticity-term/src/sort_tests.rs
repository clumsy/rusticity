//! App-level tests for the shared sort chooser: one "Sort by" section where
//! Space picks a column ascending and Space again flips its direction. The
//! choice orders the table, selection, copy and header alike.
use crate::app::{App, Service};
use crate::common::SortDirection;
use crate::keymap::{Action, Mode};
use crate::preferences::{bindings, PreferenceRow};
use crate::sort::DEFAULT_VALUE;
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

/// Press Space on the "Sort by" row `value` of the table on screen.
pub(crate) fn sort_by(app: &mut App, value: &str) {
    let model = bindings::active(app).unwrap();
    let index = model
        .rows()
        .iter()
        .position(|row| matches!(row, PreferenceRow::Sort { value: v, .. } if *v == value))
        .unwrap_or_else(|| panic!("no Sort by row {value}"));
    app.mode = Mode::ColumnSelector;
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    app.mode = Mode::Normal;
}

/// The selected "Sort by" row and its direction.
fn selected(
    model: &crate::preferences::Preferences,
) -> Option<(&'static str, Option<SortDirection>)> {
    model.rows().iter().find_map(|row| match row {
        PreferenceRow::Sort {
            value,
            selected: true,
            direction,
            ..
        } => Some((*value, *direction)),
        _ => None,
    })
}

fn sort_values(model: &crate::preferences::Preferences) -> Vec<&'static str> {
    model
        .rows()
        .iter()
        .filter_map(|row| match row {
            PreferenceRow::Sort { value, .. } => Some(*value),
            _ => None,
        })
        .collect()
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
fn one_section_shows_the_order_in_effect_with_its_arrow() {
    use crate::rds::instance::Column;
    let app = rds_app();
    assert_eq!(rds_order(&app), ["db-1", "db-2", "db-10"]);
    let model = bindings::active(&app).unwrap();
    assert_eq!(
        selected(&model),
        Some((Column::Identifier.id(), Some(SortDirection::Asc)))
    );
    assert!(!model
        .rows()
        .iter()
        .any(|r| matches!(r, PreferenceRow::Section("Sort direction"))));
    assert!(screen(&app).contains("DB identifier ↑"));
}

#[test]
fn space_picks_ascending_then_flips_and_everything_follows() {
    use crate::rds::instance::Column;
    let mut app = rds_app();
    app.rds_state.instances.selected = 2;
    sort_by(&mut app, Column::Engine.id());
    assert_eq!(
        rds_order(&app),
        ["db-1", "db-2", "db-10"],
        "aurora, mysql, postgres"
    );
    assert_eq!(
        app.rds_state.instances.selected, 0,
        "the new order starts at the top"
    );
    sort_by(&mut app, Column::Engine.id());
    assert_eq!(rds_order(&app), ["db-10", "db-2", "db-1"]);
    let model = bindings::active(&app).unwrap();
    assert_eq!(
        selected(&model),
        Some((Column::Engine.id(), Some(SortDirection::Desc)))
    );
    let text = screen(&app);
    assert!(text.contains("Engine ↓") && !text.contains("DB identifier ↑"));
    assert_eq!(
        crate::rds::actions::selected_arn(&app).as_deref(),
        Some("arn:aws:rds:us-east-2:123456789012:db:db-10"),
        "copy follows the order on screen"
    );
}

#[test]
fn hiding_the_sort_column_falls_back_to_the_default_order() {
    use crate::rds::instance::Column;
    let mut app = rds_app();
    sort_by(&mut app, Column::Engine.id());
    sort_by(&mut app, Column::Engine.id());
    app.rds_state
        .visible_column_ids
        .retain(|id| *id != Column::Engine.id());
    assert_eq!(rds_order(&app), ["db-1", "db-2", "db-10"]);
    let model = bindings::active(&app).unwrap();
    assert_eq!(selected(&model).map(|s| s.0), Some(Column::Identifier.id()));
    assert!(!sort_values(&model).contains(&Column::Engine.id()));
}

#[test]
fn every_table_offers_one_sort_section_that_toggles() {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.service_selected = true;
    for service in [
        Service::RdsInstances,
        Service::Ec2Instances,
        Service::VpcVpcs,
        Service::SecretsManagerSecrets,
        Service::AcmCertificates,
        Service::SqsQueues,
        Service::LambdaFunctions,
        Service::LambdaApplications,
        Service::DynamoDbTables,
        Service::KmsKeys,
        Service::SnsTopics,
        Service::StepFunctionsStateMachines,
        Service::EventBridgeEventBuses,
        Service::Ec2LoadBalancers,
        Service::Ec2TargetGroups,
        Service::EcrRepositories,
        Service::EfsFileSystems,
        Service::FsxFileSystems,
        Service::CloudWatchLogGroups,
        Service::ApiGatewayApis,
        Service::CloudFormationStacks,
        Service::IamUsers,
        Service::IamRoles,
    ] {
        app.current_service = service;
        app.mode = Mode::Normal;
        let model = bindings::model_for(&app, service);
        let values = sort_values(&model);
        let column = *values
            .iter()
            .find(|v| **v != DEFAULT_VALUE)
            .unwrap_or_else(|| panic!("{service:?} lists its columns"));
        let before = selected(&model).unwrap();
        sort_by(&mut app, column);
        let after = selected(&bindings::model_for(&app, service)).unwrap();
        let expected = if before.0 == column {
            // The column already in effect flips.
            match before.1 {
                Some(SortDirection::Asc) => SortDirection::Desc,
                _ => SortDirection::Asc,
            }
        } else {
            SortDirection::Asc
        };
        assert_eq!(after, (column, Some(expected)), "{service:?} first press");
        sort_by(&mut app, column);
        let again = selected(&bindings::model_for(&app, service)).unwrap();
        assert_ne!(again.1, after.1, "{service:?} second press flips");
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
    sort_by(&mut app, crate::ec2::Column::InstanceId.id());
    let ids = |app: &App| -> Vec<String> {
        crate::ui::ec2::filtered_ec2_instances(app)
            .iter()
            .map(|i| i.instance_id.clone())
            .collect()
    };
    assert_eq!(
        ids(&app),
        ["i-0a", "i-0b", "i-0c"],
        "a new column starts ascending"
    );
    app.ec2_state.table.items[0].key_name = "deploy".into();
    app.ec2_state.table.filter = "deploy".into();
    assert_eq!(ids(&app), ["i-0b"], "filter by key pair name, as on screen");
}

mod choice_tables {
    use super::*;

    #[test]
    fn default_order_comes_first_and_clears_the_choice() {
        let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
        app.service_selected = true;
        app.current_service = Service::SqsQueues;
        app.mode = Mode::Normal;
        let model = bindings::model_for(&app, Service::SqsQueues);
        assert_eq!(sort_values(&model)[0], DEFAULT_VALUE);
        assert_eq!(selected(&model), Some((DEFAULT_VALUE, None)));
        sort_by(&mut app, crate::sqs::queue::Column::Name.id());
        sort_by(&mut app, DEFAULT_VALUE);
        assert_eq!(
            selected(&bindings::model_for(&app, Service::SqsQueues)),
            Some((DEFAULT_VALUE, None))
        );
    }

    #[test]
    fn sqs_queues_follow_the_choice_and_return_to_their_order() {
        use crate::sqs::queue::Queue;
        let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.sqs_state.queues.items = ["orders-10", "orders-2", "audit"]
            .into_iter()
            .map(|name| Queue {
                name: name.into(),
                ..Default::default()
            })
            .collect();
        let order = |app: &App| -> Vec<String> {
            crate::ui::sqs::visible_queues(app)
                .iter()
                .map(|q| q.name.clone())
                .collect()
        };
        assert_eq!(
            order(&app),
            ["orders-10", "orders-2", "audit"],
            "API order kept"
        );
        sort_by(&mut app, crate::sqs::queue::Column::Name.id());
        assert_eq!(order(&app), ["audit", "orders-2", "orders-10"]);
        assert!(screen(&app).contains("Name ↑"));
        sort_by(&mut app, crate::sqs::queue::Column::Name.id());
        assert_eq!(order(&app), ["orders-10", "orders-2", "audit"]);
        app.sqs_visible_column_ids
            .retain(|id| *id != crate::sqs::queue::Column::Name.id());
        assert_eq!(
            order(&app),
            ["orders-10", "orders-2", "audit"],
            "hidden: own order"
        );
    }

    #[test]
    fn eventbridge_rules_sort_by_their_cells() {
        let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
        app.config.account_id = "123456789012".into();
        app.service_selected = true;
        app.current_service = Service::EventBridgeEventBuses;
        app.mode = Mode::Normal;
        app.eventbridge_state.buses.rows.items = vec![rusticity_core::EventBus {
            name: "default".into(),
            arn: "arn:aws:events:us-east-2:123456789012:event-bus/default".into(),
            ..Default::default()
        }];
        app.handle_action(Action::Select);
        app.eventbridge_state.bus.as_mut().unwrap().rules.rows.items = ["zeta", "alpha", "mid"]
            .into_iter()
            .map(|name| rusticity_core::EventRule {
                name: name.into(),
                arn: format!("arn:aws:events:us-east-2:123456789012:rule/{name}"),
                ..Default::default()
            })
            .collect();
        sort_by(&mut app, "Name");
        sort_by(&mut app, "Name");
        let names: Vec<_> = app
            .eventbridge_state
            .bus
            .as_ref()
            .unwrap()
            .rules
            .filtered()
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, ["zeta", "mid", "alpha"]);
        assert!(screen(&app).contains("Name ↓"));
    }
}
