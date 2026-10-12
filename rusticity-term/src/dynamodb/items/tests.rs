use super::*;
use crate::common::{InputFocus, PageSize};
use crate::dynamodb::actions::{go_to_page, next_dropdown_value};
use crate::keymap::{handle_key, Action};
use aws_sdk_dynamodb::types::AttributeValue;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusticity_core::dynamodb::{DdbIndex, DdbItem, DdbTable};

fn cursor(value: &str) -> DdbItemCursor {
    [("pk".into(), AttributeValue::S(value.into()))]
        .into_iter()
        .collect()
}

fn item(value: &str) -> DdbItem {
    DdbItem {
        attributes: [("pk".into(), value.into())].into_iter().collect(),
    }
}

fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    app.current_service = Service::DynamoDbTables;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.log_groups_state.loading = false;
    app.dynamodb_state.current_table = Some("orders".into());
    app.dynamodb_state.tables.items = vec![DdbTable {
        name: "orders".into(),
        partition_key: "pk (S)".into(),
        global_secondary_indexes: vec![DdbIndex {
            name: "by-customer".into(),
            partition_key: "customer_id (N)".into(),
            ..Default::default()
        }],
        ..Default::default()
    }];
    let request = context(&app).unwrap();
    app.dynamodb_state.begin_items_request(request);
    app.dynamodb_state.finish_items_request(DdbItemsPage {
        items: vec![item("first"), item("second")],
        next_cursor: Some(cursor("next")),
    });
    app
}

fn render(app: &App) -> String {
    use ratatui::{backend::TestBackend, Terminal};
    let mut terminal = Terminal::new(TestBackend::new(220, 45)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn load_more_is_explicit_and_preserves_selection_expansion_and_columns() {
    let mut app = app();
    app.dynamodb_state.items.selected = 1;
    app.dynamodb_state.items.expanded_item = Some(0);
    app.dynamodb_state.items.page_size = PageSize::Ten;
    app.dynamodb_state.item_hidden_columns = vec!["pk".into()];
    assert!(!needs_load(&app));
    app.handle_action(Action::PageDown);
    assert!(!needs_load(&app));
    assert!(render(&app).contains("m load more"));
    app.handle_action(Action::LoadMore);
    assert!(app.dynamodb_state.items_loading);
    assert!(!can_load_more(&app));
    assert!(render(&app).contains("first"));
    assert!(render(&app).contains("[1] ..."));
    // A second activation while queued cannot enqueue another request.
    app.handle_action(Action::LoadMore);
    let request = context(&app).unwrap();
    assert_eq!(
        app.dynamodb_state.begin_items_request(request),
        Some(cursor("next"))
    );
    let mut next = item("third");
    next.attributes
        .insert("new_attribute".into(), "value".into());
    app.dynamodb_state.finish_items_request(DdbItemsPage {
        items: vec![next],
        next_cursor: None,
    });
    assert_eq!(app.dynamodb_state.items.items.len(), 3);
    assert_eq!(app.dynamodb_state.items.selected, 1);
    assert_eq!(app.dynamodb_state.items.expanded_item, Some(0));
    assert_eq!(app.dynamodb_state.item_hidden_columns, ["pk"]);
    assert!(
        dynamodb::item_column_names(dynamodb::current_table(&app).unwrap(), &app)
            .contains(&"new_attribute".into())
    );
    assert!(!can_load_more(&app));
    assert!(!needs_load(&app));
    assert!(results_complete(&app));
    app.handle_action(Action::LoadMore);
    assert!(!app.dynamodb_state.items_loading);
}

#[test]
fn continuation_survives_empty_pages_and_zero_local_matches() {
    let mut app = app();
    app.dynamodb_state.item_filter = "absent".into();
    assert!(dynamodb::filtered_items(&app).is_empty());
    assert!(can_load_more(&app));
    app.handle_action(Action::LoadMore);
    app.dynamodb_state
        .begin_items_request(context(&app).unwrap());
    app.dynamodb_state.finish_items_request(DdbItemsPage {
        items: vec![],
        next_cursor: Some(cursor("after-empty")),
    });
    assert_eq!(app.dynamodb_state.items.items.len(), 2);
    assert!(can_load_more(&app));
    assert!(!needs_load(&app));
    assert!(dynamodb::filtered_items(&app).is_empty());
    assert!(render(&app).contains("m load more"));
    app.handle_action(Action::LoadMore);
    assert_eq!(
        app.dynamodb_state
            .begin_items_request(context(&app).unwrap()),
        Some(cursor("after-empty"))
    );
}

#[test]
fn failure_keeps_rows_and_cursor_and_retries_the_same_page() {
    let mut app = app();
    app.dynamodb_state.items.selected = 1;
    app.handle_action(Action::LoadMore);
    let request = context(&app).unwrap();
    let key = app.dynamodb_state.begin_items_request(request.clone());
    app.dynamodb_state.fail_items_request();
    assert_eq!(app.dynamodb_state.items.items.len(), 2);
    assert_eq!(app.dynamodb_state.items.selected, 1);
    assert_eq!(app.dynamodb_state.items_cursor, key);
    assert!(!needs_load(&app));
    assert!(app.dynamodb_state.items_failed);
    assert!(render(&app).contains("m retry load more"));
    app.mode = Mode::ErrorModal;
    app.error_message = Some("throttled".into());
    app.handle_action(Action::RetryLoad);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.error_message.is_none());
    assert!(!app.log_groups_state.loading);
    assert!(needs_load(&app));
    assert_eq!(app.dynamodb_state.begin_items_request(request), key);
    app.dynamodb_state.finish_items_request(DdbItemsPage {
        items: vec![item("third")],
        next_cursor: None,
    });
    assert_eq!(app.dynamodb_state.items.items.len(), 3);
    assert!(!app.dynamodb_state.items_failed);
}

#[test]
fn all_request_context_changes_reject_the_previous_cursor() {
    let base = context(&app()).unwrap();
    let mut changed = Vec::new();
    let mut request = base.clone();
    request.profile = "other".into();
    changed.push(request);
    let mut request = base.clone();
    request.region = "eu-west-1".into();
    changed.push(request);
    let mut request = base.clone();
    request.account_id = "other".into();
    changed.push(request);
    let mut request = base.clone();
    request.role_arn = "other".into();
    changed.push(request);
    let mut request = base.clone();
    request.table = "other".into();
    changed.push(request);
    let mut request = base.clone();
    request.index = Some("index".into());
    changed.push(request);
    let mut request = base.clone();
    request.keys = vec!["new-key".into()];
    changed.push(request);
    let mut request = base.clone();
    request.partition_type = "N".into();
    changed.push(request);
    let mut request = base.clone();
    request.operation = ItemOperation::Query;
    changed.push(request);
    let mut request = base.clone();
    request.projection = ItemProjection::KeysOnly;
    changed.push(request);
    let mut request = base.clone();
    request.query = Some("new-value".into());
    changed.push(request);
    for request in changed {
        let mut app = app();
        app.dynamodb_state.items_append = true;
        app.dynamodb_state.items.selected = 1;
        app.dynamodb_state.items.expanded_item = Some(1);
        assert_eq!(app.dynamodb_state.begin_items_request(request), None);
        assert!(app.dynamodb_state.items.items.is_empty());
        assert_eq!(app.dynamodb_state.items_cursor, None);
        assert_eq!(app.dynamodb_state.items.selected, 0);
        assert_eq!(app.dynamodb_state.items.expanded_item, None);
        assert!(!app.dynamodb_state.items_append);
    }
}

#[test]
fn dropdowns_and_refresh_reset_cursor_but_local_filter_and_page_size_do_not() {
    for focus in [
        dynamodb::ITEM_OPERATION,
        dynamodb::ITEM_INDEX,
        dynamodb::ITEM_PROJECTION,
    ] {
        let mut app = app();
        app.dynamodb_state.input_focus = focus;
        next_dropdown_value(&mut app);
        assert!(app.dynamodb_state.items_cursor.is_none());
        assert!(app.dynamodb_state.items_context.is_none());
        assert!(app.dynamodb_state.items.items.is_empty());
    }
    let mut app = app();
    app.dynamodb_state.item_filter = "second".into();
    app.dynamodb_state.items.set_page_size(PageSize::Ten);
    assert!(can_load_more(&app));
    assert!(!needs_load(&app));
    app.handle_action(Action::Refresh);
    assert!(app.dynamodb_state.items_loading);
    assert!(app.dynamodb_state.items_cursor.is_none());
    assert!(app.dynamodb_state.items.items.is_empty());
    assert!(!app.log_groups_state.loading);
}

#[test]
fn live_context_changes_disable_load_more_and_schedule_a_fresh_request() {
    let mut app = app();
    app.profile = "other-profile".into();
    assert!(!can_load_more(&app));
    assert!(needs_load(&app));
    assert!(!results_complete(&app));
    app.handle_action(Action::LoadMore);
    assert!(!app.dynamodb_state.items_append);
}

#[test]
fn failed_query_does_not_automatically_retry_on_navigation() {
    let mut app = app();
    app.dynamodb_state.item_operation = ItemOperation::Query;
    app.dynamodb_state.item_filter = "customer".into();
    assert!(needs_load(&app));
    app.dynamodb_state
        .begin_items_request(context(&app).unwrap());
    app.dynamodb_state.fail_items_request();
    assert!(!needs_load(&app));
    app.handle_action(Action::NextItem);
    assert!(!needs_load(&app));
    app.dynamodb_state.item_filter = "another".into();
    app.mode = Mode::FilterInput;
    assert!(!needs_load(&app));
    app.handle_action(Action::ApplyFilter);
    assert!(needs_load(&app));
}

#[tokio::test]
async fn clearing_query_and_missing_table_clear_old_rows_without_aws_requests() {
    let mut app = app();
    app.dynamodb_state.item_operation = ItemOperation::Query;
    app.dynamodb_state.item_filter = "customer".into();
    app.dynamodb_state
        .begin_items_request(context(&app).unwrap());
    app.dynamodb_state.finish_items_request(DdbItemsPage {
        items: vec![item("old")],
        next_cursor: Some(cursor("old")),
    });
    app.dynamodb_state.item_filter.clear();
    assert!(needs_load(&app));
    // Dummy clients have no credentials. This succeeds only if no request is sent.
    app.load_dynamodb_items().await.unwrap();
    assert!(app.dynamodb_state.items.items.is_empty());
    assert!(app.dynamodb_state.items_cursor.is_none());
    assert!(!needs_load(&app));
    app.dynamodb_state.tables.items.clear();
    app.dynamodb_state.items_loading = true;
    app.load_dynamodb_items().await.unwrap();
    assert!(!app.dynamodb_state.items_loading);
}

#[test]
fn load_more_key_is_normal_mode_only_and_ignored_outside_items() {
    let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(handle_key(key, Mode::Normal), Some(Action::LoadMore));
    assert_eq!(
        handle_key(key, Mode::FilterInput),
        Some(Action::FilterInput('m'))
    );
    assert_eq!(handle_key(key, Mode::ColumnSelector), None);
    let mut app = app();
    app.dynamodb_state.detail_tab = dynamodb::DetailTab::Settings;
    app.handle_action(Action::LoadMore);
    assert!(!app.dynamodb_state.items_loading);
    app.dynamodb_state.detail_tab = dynamodb::DetailTab::Items;
    app.current_service = Service::EcrRepositories;
    app.handle_action(Action::LoadMore);
    assert!(!app.dynamodb_state.items_loading);
    app.current_service = Service::DynamoDbTables;
    app.mode = Mode::FilterInput;
    app.dynamodb_state.input_focus = InputFocus::Filter;
    app.handle_action(Action::LoadMore);
    assert!(!app.dynamodb_state.items_loading);
}

#[test]
fn pagination_controls_only_navigate_loaded_items() {
    let mut app = app();
    app.dynamodb_state.items.items = (0..35).map(|i| item(&format!("item-{i}"))).collect();
    app.dynamodb_state.items.page_size = PageSize::Ten;
    let key = app.dynamodb_state.items_cursor.clone();
    go_to_page(&mut app, 3);
    assert_eq!(app.dynamodb_state.items.selected, 20);
    app.mode = Mode::FilterInput;
    app.dynamodb_state.input_focus = InputFocus::Pagination;
    app.handle_action(Action::PageDown);
    assert_eq!(app.dynamodb_state.items.selected, 30);
    app.handle_action(Action::PageUp);
    assert_eq!(app.dynamodb_state.items.selected, 20);
    assert_eq!(app.dynamodb_state.tables.selected, 0);
    assert_eq!(app.dynamodb_state.items_cursor, key);
    app.mode = Mode::Normal;
    assert!(!needs_load(&app));
    app.dynamodb_state.items.items.clear();
    assert!(can_load_more(&app));
    assert!(render(&app).contains("All attributes ⋮ ..."));
}

#[test]
fn pagination_stays_unknown_through_loading_and_failure_until_exhausted() {
    let mut app = app();
    app.dynamodb_state.items.items = (0..100).map(|i| item(&format!("item-{i}"))).collect();
    app.dynamodb_state.items.page_size = PageSize::Fifty;
    assert!(!results_complete(&app));
    assert!(render(&app).contains("[1] 2 ..."));
    assert!(!render(&app).contains("[1] 2 3"));

    app.handle_action(Action::LoadMore);
    assert!(!can_load_more(&app));
    assert!(!results_complete(&app));
    assert!(render(&app).contains("[1] 2 ..."));
    app.dynamodb_state
        .begin_items_request(context(&app).unwrap());
    app.dynamodb_state.fail_items_request();
    assert!(render(&app).contains("[1] 2 ..."));

    retry(&mut app);
    app.dynamodb_state
        .begin_items_request(context(&app).unwrap());
    app.dynamodb_state.finish_items_request(DdbItemsPage {
        items: (100..152).map(|i| item(&format!("item-{i}"))).collect(),
        next_cursor: None,
    });
    assert!(results_complete(&app));
    let screen = render(&app);
    assert!(screen.contains("[1] 2 3 4"));
    assert!(!screen.contains("[1] 2 3 4 ..."));
}

#[test]
fn pagination_with_no_loaded_matches_does_not_claim_an_empty_final_result() {
    let mut app = app();
    app.dynamodb_state.item_filter = "no-match".into();
    assert!(!results_complete(&app));
    assert!(render(&app).contains("All attributes ⋮ ..."));
    app.dynamodb_state.items.items.clear();
    assert!(render(&app).contains("All attributes ⋮ ..."));
    app.dynamodb_state
        .finish_items_request(DdbItemsPage::default());
    assert!(results_complete(&app));
    assert!(render(&app).contains("All attributes ⋮ [1]"));
    assert!(!render(&app).contains("All attributes ⋮ ..."));
}

#[test]
fn unstarted_failed_loading_or_stale_results_never_claim_a_known_total() {
    let mut app = app();
    app.dynamodb_state
        .finish_items_request(DdbItemsPage::default());
    assert!(results_complete(&app));
    app.dynamodb_state.items_failed = true;
    assert!(!results_complete(&app));
    app.dynamodb_state.items_failed = false;
    app.dynamodb_state.items_loading = true;
    assert!(!results_complete(&app));
    app.dynamodb_state.items_loading = false;
    app.profile = "changed".into();
    assert!(!results_complete(&app));
    app.profile = "test".into();
    assert!(results_complete(&app));
    app.dynamodb_state.clear_items_result();
    assert!(!results_complete(&app));
    assert!(render(&app).contains("All attributes ⋮ ..."));
    app.dynamodb_state.item_operation = ItemOperation::Query;
    app.dynamodb_state
        .begin_items_request(context(&app).unwrap());
    app.dynamodb_state
        .finish_items_request(DdbItemsPage::default());
    assert!(
        !results_complete(&app),
        "An empty query key has not executed a query"
    );
    assert!(render(&app).contains("All attributes ⋮ ..."));
}

#[test]
fn result_summary_is_absent_in_partial_loading_failed_and_complete_states() {
    let mut app = app();
    for stage in 0..4 {
        match stage {
            1 => app.handle_action(Action::LoadMore),
            2 => app.dynamodb_state.fail_items_request(),
            3 => app
                .dynamodb_state
                .finish_items_request(DdbItemsPage::default()),
            _ => {}
        }
        let screen = render(&app);
        for text in [
            " loaded,",
            " matching",
            "No more pages",
            "Loading next page",
            "Read failed;",
        ] {
            assert!(
                !screen.contains(text),
                "Unexpected summary in stage {stage}: {text}"
            );
        }
        if stage < 3 {
            assert!(screen.contains("[1] ..."));
        } else {
            assert!(results_complete(&app));
            assert!(!screen.contains("[1] ..."));
        }
    }
}
