use super::*;
use crate::{
    app::{Service, Tab},
    keymap::handle_key,
    tab_runtime::TabId,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusticity_core::{
    finder::{Resource, ResourceKind},
    SnsTopic,
};
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from("/tmp").join(format!(
            "rusticity-bookmark-ui-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("bookmarks.json")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app(temp: &Temp) -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::SnsTopics;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.bookmarks.path = Some(temp.file());
    app.sns_state.topics.items = ["other", "chosen"]
        .into_iter()
        .map(|name| {
            SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}")).unwrap()
        })
        .collect();
    app.sns_state.topics.filter = "chosen".into();
    app.tabs.push(Tab {
        id: TabId::new(),
        service: Service::SnsTopics,
        title: Service::SnsTopics.name().into(),
        breadcrumb: Service::SnsTopics.name().into(),
    });
    app
}

fn press(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    if let Some(action) = handle_key(KeyEvent::new(key, modifiers), app.mode) {
        app.handle_action(action);
    }
}

fn screen(app: &App, width: u16, height: u16) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
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
fn toggle_uses_filtered_resource_and_preserves_source_without_network() {
    let temp = Temp::new();
    let mut app = app(&temp);
    press(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    let rows = store::load(&temp.file()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].resource.id, "chosen");
    assert_eq!(rows[0].profile, "test");
    assert_eq!(rows[0].region, "us-east-1");
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.sns_state.topics.filter, "chosen");
    assert!(!app.sns_state.inventory.is_busy());
    assert!(app.bookmarks.notice().unwrap().0.contains("bookmarked"));
    press(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert!(store::load(&temp.file()).unwrap().is_empty());
}

#[test]
fn finder_pins_only_a_selected_supported_result() {
    let temp = Temp::new();
    let mut app = app(&temp);
    app.mode = Mode::ResourceFinder;
    app.resource_finder.rows = vec![Resource {
        kind: ResourceKind::Topic,
        id: "found".into(),
        name: "found".into(),
        arn: "arn:aws:sns:us-east-1:123456789012:found".into(),
        region: "us-east-1".into(),
    }];
    press(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert!(!temp.file().exists());
    app.resource_finder.focus = crate::finder::Focus::Table;
    press(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(store::load(&temp.file()).unwrap()[0].resource.id, "found");
    assert!(app.resource_finder.pending_open.is_none());
    app.resource_finder.rows[0].kind = ResourceKind::Reference;
    press(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert!(app.bookmarks.notice().unwrap().1);
    assert_eq!(store::load(&temp.file()).unwrap().len(), 1);
}

#[test]
fn picker_filters_opens_expands_and_deletes_only_the_visible_bookmark() {
    let temp = Temp::new();
    let mut app = app(&temp);
    app.handle_action(Action::ToggleBookmark);
    app.sns_state.topics.filter = "other".into();
    app.handle_action(Action::ToggleBookmark);
    press(&mut app, KeyCode::Char(' '), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::BookmarkPicker);
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    for c in "other".chars() {
        press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
    }
    assert_eq!(app.bookmarks.filtered().len(), 1);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(!app.bookmarks.filtering);
    assert!(app.bookmarks.pending_open.is_none());
    press(&mut app, KeyCode::Right, KeyModifiers::NONE);
    assert!(screen(&app, 180, 30).contains("ARN:"));
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        app.bookmarks.pending_open.take().unwrap().resource.id,
        "other"
    );
    app.bookmarks.opening = false;
    press(&mut app, KeyCode::Delete, KeyModifiers::NONE);
    let remaining = store::load(&temp.file()).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].resource.id, "chosen");
    for (width, height) in [(1, 1), (30, 8), (180, 30)] {
        screen(&app, width, height);
    }
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn corrupt_store_is_reported_and_never_replaced_by_a_toggle() {
    let temp = Temp::new();
    let mut app = app(&temp);
    std::fs::write(temp.file(), "not json").unwrap();
    app.handle_action(Action::ToggleBookmark);
    assert!(app.bookmarks.notice().unwrap().1);
    assert_eq!(std::fs::read_to_string(temp.file()).unwrap(), "not json");
    assert_eq!(app.mode, Mode::Normal);
}

#[tokio::test]
async fn same_context_reopens_through_finder_and_selects_exact_topic_after_local_completion() {
    let temp = Temp::new();
    let mut app = app(&temp);
    let entry = selected(&app).unwrap();
    app.bookmarks.pending_open = Some(entry);
    open_with(&mut app, |_, _| async {
        panic!("same context must not reconnect")
    })
    .await
    .unwrap();
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(
        app.sns_state.pending_focus.as_deref(),
        Some("arn:aws:sns:us-east-1:123456789012:chosen")
    );
    assert!(app.sns_state.topics.filter.is_empty());
    assert!(app.sns_state.open_after_load);
    assert!(app.sns_state.inventory.is_pending());
    let rows = ["achosen", "chosen"]
        .into_iter()
        .map(|name| {
            SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}")).unwrap()
        })
        .collect();
    assert!(app.sns_state.inventory.start(async { Ok(rows) }));
    tokio::task::yield_now().await;
    crate::sns::inventory::poll(&mut app).await;
    assert_eq!(app.sns_state.topics.selected, 1);
    assert_eq!(
        app.sns_state.subscriptions.topic.as_ref().unwrap().name,
        "chosen"
    );
    assert!(app.sns_state.topics.filter.is_empty());
    assert_eq!(app.mode, Mode::Normal);
    app.handle_action(Action::PrevTab);
    assert_eq!(app.sns_state.topics.filter, "chosen");
    assert!(app.sns_state.subscriptions.topic.is_none());
}

#[tokio::test]
async fn cross_context_auth_failure_or_account_mismatch_preserves_current_app() {
    let temp = Temp::new();
    for wrong_account in [false, true] {
        let mut app = app(&temp);
        let mut entry = selected(&app).unwrap();
        entry.profile = "other-profile".into();
        app.bookmarks.pending_open = Some(entry);
        let old_id = app.tabs[0].id;
        let result = open_with(&mut app, |profile, region| async move {
            if !wrong_account {
                anyhow::bail!("authentication failed");
            }
            let mut replacement = App::new_without_client(profile, Some(region));
            replacement.config.account_id = "999999999999".into();
            Ok(replacement)
        })
        .await;
        assert!(result.is_err());
        assert_eq!(app.profile, "test");
        assert_eq!(app.tabs[0].id, old_id);
        assert_eq!(app.sns_state.topics.filter, "chosen");
    }
}

#[tokio::test]
async fn cross_profile_open_uses_saved_profile_and_preserves_bookmark_store() {
    let temp = Temp::new();
    let mut app = app(&temp);
    let mut entry = selected(&app).unwrap();
    entry.profile = "other-profile".into();
    app.bookmarks.pending_open = Some(entry);
    open_with(&mut app, |profile, region| async move {
        assert_eq!(profile, "other-profile");
        assert_eq!(region, "us-east-1");
        let mut replacement = App::new_without_client(profile, Some(region));
        replacement.config.account_id = "123456789012".into();
        Ok(replacement)
    })
    .await
    .unwrap();
    assert_eq!(app.profile, "other-profile");
    assert_eq!(app.bookmarks.path.as_ref(), Some(&temp.file()));
    assert!(app.sns_state.inventory.is_pending());
}

#[test]
fn bookmark_shortcuts_do_not_hijack_typing_or_the_existing_tab_picker() {
    let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(handle_key(key, Mode::Normal), Some(Action::ToggleBookmark));
    assert_eq!(
        handle_key(key, Mode::ResourceFinder),
        Some(Action::ToggleBookmark)
    );
    assert_eq!(handle_key(key, Mode::FilterInput), None);
    assert_eq!(handle_key(key, Mode::BookmarkPicker), None);
    assert_eq!(
        handle_key(
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
            Mode::SpaceMenu
        ),
        Some(Action::OpenTabPicker)
    );
    assert_eq!(
        handle_key(
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            Mode::SpaceMenu
        ),
        Some(Action::OpenBookmarkPicker)
    );
}

#[test]
fn bookmarking_and_status_hint_require_a_dedicated_view() {
    let temp = Temp::new();
    let mut app = app(&temp);
    app.mode = Mode::ResourceFinder;
    app.resource_finder.focus = crate::finder::Focus::Table;
    for kind in ResourceKind::ALL {
        app.resource_finder.rows = vec![Resource {
            kind,
            id: "resource-id".into(),
            name: "resource-name".into(),
            arn: String::new(),
            region: "us-east-1".into(),
        }];
        let expected = kind != ResourceKind::Vpc;
        assert_eq!(available(&app), expected, "{kind:?}");
        assert_eq!(
            screen(&app, 160, 30).contains("^b bookmark"),
            expected,
            "{kind:?}"
        );
        if !expected {
            app.handle_action(Action::ToggleBookmark);
            assert!(!temp.file().exists());
            assert!(app.bookmarks.notice().unwrap().1);
            app.bookmarks.notice = None;
        }
    }
}

#[tokio::test]
async fn legacy_inventory_only_bookmark_is_refused_before_reconnecting() {
    let temp = Temp::new();
    let mut app = app(&temp);
    let mut entry = selected(&app).unwrap();
    entry.resource.kind = ResourceKind::Vpc;
    entry.resource.arn.clear();
    entry.profile = "other-profile".into();
    app.bookmarks.pending_open = Some(entry);
    let error = open_with(&mut app, |_, _| async {
        panic!("unsupported bookmark must not connect")
    })
    .await
    .unwrap_err();
    assert!(error.to_string().contains("no dedicated resource view"));
    assert_eq!(app.profile, "test");
    assert_eq!(app.tabs.len(), 1);
}

#[tokio::test]
async fn two_bookmarks_from_one_inventory_open_independent_exact_views() {
    let temp = Temp::new();
    let mut app = app(&temp);
    let first = selected(&app).unwrap();
    app.sns_state.topics.filter = "other".into();
    let second = selected(&app).unwrap();
    let source_filter = app.sns_state.topics.filter.clone();
    let mut ids = Vec::new();
    for entry in [first, second] {
        app.bookmarks.pending_open = Some(entry);
        assert!(!open_pending(&mut app).await.unwrap());
        ids.push(app.tabs[app.current_tab].id);
        let rows = ["chosen-suffix", "chosen", "other"]
            .into_iter()
            .map(|name| {
                SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}")).unwrap()
            })
            .collect();
        assert!(app.sns_state.inventory.start(async { Ok(rows) }));
        tokio::task::yield_now().await;
        crate::sns::inventory::poll(&mut app).await;
        assert!(app.sns_state.topics.filter.is_empty());
    }
    assert_ne!(ids[0], ids[1]);
    assert_eq!(
        app.sns_state.subscriptions.topic.as_ref().unwrap().name,
        "other"
    );
    app.handle_action(Action::PrevTab);
    assert_eq!(
        app.sns_state.subscriptions.topic.as_ref().unwrap().name,
        "chosen"
    );
    app.handle_action(Action::PrevTab);
    assert_eq!(app.sns_state.topics.filter, source_filter);
    assert!(app.sns_state.subscriptions.topic.is_none());
}

#[tokio::test]
async fn missing_bookmark_target_never_opens_a_similarly_named_resource() {
    let temp = Temp::new();
    let mut app = app(&temp);
    app.bookmarks.pending_open = Some(selected(&app).unwrap());
    open_pending(&mut app).await.unwrap();
    let row = SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:chosen-suffix").unwrap();
    app.sns_state.inventory.start(async { Ok(vec![row]) });
    tokio::task::yield_now().await;
    crate::sns::inventory::poll(&mut app).await;
    assert!(app.sns_state.subscriptions.topic.is_none());
    assert!(app.sns_state.topics.items.is_empty());
    assert_eq!(
        app.sns_state.load_status(),
        crate::async_read::ReadStatus::Failed
    );
    assert!(screen(&app, 160, 30).contains("Resource not found or not accessible"));
}

#[tokio::test]
async fn explicit_cancel_or_refresh_drops_the_pending_jump() {
    for action in [Action::GoBack, Action::Refresh] {
        let temp = Temp::new();
        let mut app = app(&temp);
        app.bookmarks.pending_open = Some(selected(&app).unwrap());
        open_pending(&mut app).await.unwrap();
        assert!(app.sns_state.open_after_load);
        app.handle_action(action);
        assert!(!app.sns_state.open_after_load);
        assert!(app.sns_state.pending_focus.is_none());
        assert!(app.sns_state.subscriptions.topic.is_none());
    }
}

#[tokio::test]
async fn deferred_entry_preserves_active_input_mode_and_page_text() {
    let temp = Temp::new();
    let mut app = app(&temp);
    app.bookmarks.pending_open = Some(selected(&app).unwrap());
    open_pending(&mut app).await.unwrap();
    let topic = SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:chosen").unwrap();
    app.sns_state.inventory.start(async { Ok(vec![topic]) });
    tokio::task::yield_now().await;
    app.mode = Mode::FilterInput;
    app.page_input = "27".into();
    crate::sns::inventory::poll(&mut app).await;
    assert_eq!(app.mode, Mode::FilterInput);
    assert_eq!(app.page_input, "27");
    assert_eq!(
        app.sns_state.subscriptions.topic.as_ref().unwrap().name,
        "chosen"
    );
}
