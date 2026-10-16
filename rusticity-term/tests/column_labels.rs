//! Separate integration-test process: the immutable application-wide i18n map
//! can be populated here without racing unit tests that expect default labels.
use ratatui::{backend::TestBackend, prelude::*};
use rusticity_core::{sns::SnsSubscription, SecretMetadata, SnsTopic};
use rusticity_term::{
    common::{set_i18n, SortDirection},
    secretsmanager::columns::Column as SecretColumn,
    sns::{columns::Column as TopicColumn, subscription::Column as SubscriptionColumn},
    ui::table::{expanded_from_columns, render_table, Column, OwnedColumn, TableConfig},
};
use std::{borrow::Cow, collections::HashMap, rc::Rc, sync::Once};

fn translations() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        set_i18n(HashMap::from([
            (
                "test.column.translated".into(),
                "秘密の名前 très long".into(),
            ),
            ("test.column.huge".into(), "界".repeat(70_000)),
            ("test.column.empty".into(), String::new()),
            ("column.sns.topic.name".into(), "Topic custom label".into()),
            (
                "column.sns.subscription.status".into(),
                "Delivery state".into(),
            ),
            (
                "column.secretsmanager.secret.name".into(),
                "Secret custom label".into(),
            ),
            (
                "column.ecr.repo.name".into(),
                "Repository custom label".into(),
            ),
            (
                "column.lambda.function.name".into(),
                "Function custom label".into(),
            ),
        ]));
    });
}

#[test]
fn owned_adapter_borrows_one_label_and_preserves_render_policy() {
    translations();
    let owner = Rc::new(());
    let weak = Rc::downgrade(&owner);
    let col = OwnedColumn::new(
        "test.column.translated",
        "Default",
        3,
        true,
        move |s: &String| {
            let _keep_alive = &owner;
            (s.clone(), Style::default().fg(Color::Green))
        },
    );
    assert_eq!(col.id(), "test.column.translated");
    assert_eq!(col.default_name(), "Default");
    assert_eq!(col.name(), "秘密の名前 très long");
    assert_eq!(col.width(), "秘密の名前 très long".chars().count() as u16);
    assert!(col.fit_content());
    let ptr = col.name().as_ptr();
    for _ in 0..1000 {
        let label = col.name();
        assert!(matches!(label, Cow::Borrowed(_)));
        assert_eq!(label.as_ptr(), ptr);
    }
    assert_eq!(
        col.render(&"value".into()),
        ("value".into(), Style::default().fg(Color::Green))
    );
    drop(col);
    assert!(
        weak.upgrade().is_none(),
        "dropping an adapter must release its renderer too"
    );
}

#[test]
fn owned_adapter_missing_empty_and_oversized_labels_are_bounded() {
    translations();
    fn render(_: &()) -> (String, Style) {
        (String::new(), Style::default())
    }
    let fallback = OwnedColumn::new("test.column.missing", "Fallback", 20, false, render);
    assert_eq!(fallback.name(), "Fallback");
    assert_eq!(fallback.width(), 20);
    assert!(!fallback.fit_content());
    let empty = OwnedColumn::new("test.column.empty", "Fallback", 3, false, render);
    assert_eq!(empty.name(), "");
    assert_eq!(empty.width(), 3);
    let huge = OwnedColumn::new("test.column.huge", "Fallback", 3, false, render);
    assert_eq!(huge.width(), u16::MAX);
}

#[test]
fn existing_translated_and_static_columns_return_reclaimable_labels() {
    translations();
    use rusticity_term::{apig::resource, ecr::repo, lambda::FunctionColumn};
    // Legacy explicit translation and the shared default name implementation.
    let repo: Box<dyn Column<repo::Repository>> = Box::new(repo::Column::Name);
    let function: Box<dyn Column<rusticity_term::lambda::Function>> =
        Box::new(FunctionColumn::Name);
    for _ in 0..1000 {
        assert!(matches!(repo.name(), Cow::Owned(_)));
        assert_eq!(repo.name(), "Repository custom label");
        assert!(matches!(function.name(), Cow::Owned(_)));
        assert_eq!(function.name(), "Function custom label");
    }
    let resource: Box<dyn Column<resource::Resource>> = Box::new(resource::Column::Path);
    assert!(matches!(resource.name(), Cow::Borrowed("Resource")));

    struct Missing;
    impl Column<()> for Missing {
        fn id(&self) -> &'static str {
            "test.column.missing"
        }
        fn default_name(&self) -> &'static str {
            "Fallback"
        }
        fn width(&self) -> u16 {
            12
        }
        fn render(&self, _: &()) -> (String, Style) {
            (String::new(), Style::default())
        }
    }
    assert!(matches!(Missing.name(), Cow::Borrowed("Fallback")));
}

#[test]
fn migrated_service_adapters_keep_schema_labels_values_and_compact_policy() {
    translations();
    let topic = SnsTopic::default();
    for schema in TopicColumn::ALL {
        let column = schema.into_table_column();
        assert_eq!(column.id(), schema.id());
        assert_eq!(column.name(), schema.name());
        assert_eq!(column.render(&topic), schema.render(&topic));
        assert_eq!(column.fit_content(), schema != TopicColumn::Arn);
        assert!(matches!(column.name(), Cow::Borrowed(_)));
    }
    let subscription = SnsSubscription {
        status: "Pending confirmation".into(),
        ..Default::default()
    };
    for schema in SubscriptionColumn::ALL {
        let column = schema.into_table_column();
        assert_eq!(column.id(), schema.id());
        assert_eq!(column.name(), schema.name());
        assert_eq!(column.render(&subscription), schema.render(&subscription));
        assert_eq!(
            column.fit_content(),
            !matches!(
                schema,
                SubscriptionColumn::Arn | SubscriptionColumn::Endpoint
            )
        );
    }
    let secret = SecretMetadata {
        name: "example".into(),
        last_accessed_date: "2026-10-16".into(),
        ..Default::default()
    };
    for schema in SecretColumn::ALL {
        let column = schema.into_table_column();
        assert_eq!(column.id(), schema.id());
        assert_eq!(column.name(), schema.name());
        assert_eq!(column.render(&secret), schema.render(&secret));
        assert_eq!(column.fit_content(), schema != SecretColumn::Description);
    }
}

#[test]
fn custom_labels_survive_repeated_table_and_expanded_row_rendering() {
    translations();
    let row = SecretMetadata {
        name: "example".into(),
        ..Default::default()
    };
    let columns = vec![SecretColumn::Name.into_table_column()];
    let expanded = expanded_from_columns(&columns, &row);
    assert_eq!(expanded[0].0, "Secret custom label: example");
    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
    let mut baseline = None;
    for _ in 0..100 {
        terminal
            .draw(|frame| {
                render_table(
                    frame,
                    TableConfig {
                        items: vec![&row],
                        selected_index: 0,
                        expanded_index: Some(0),
                        columns: &columns,
                        sort_column: "Secret custom label",
                        sort_direction: SortDirection::Asc,
                        title: "Secrets".into(),
                        area: frame.area(),
                        get_expanded_content: Some(Box::new(|item| {
                            expanded_from_columns(&columns, item)
                        })),
                        is_active: true,
                    },
                )
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Secret custom label"));
        assert!(text.contains("Secret custom label: example"));
        if let Some(expected) = &baseline {
            assert_eq!(buffer, expected);
        } else {
            baseline = Some(buffer.clone());
        }
    }
}
