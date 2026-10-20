//! Tests for per-tab state ownership.
//!
//! These exercise the public [`super`] API against the agreed managed-service
//! surface (`crate::sns::inventory::{request,poll}` and `SnsState::load_status`,
//! supplied by the inventory worker). They will not build until that surface
//! lands; that dependency is intentional and documented in the integration
//! report, not an error in this module.

use super::*;
use crate::app::{App, Service, Tab};

fn app() -> App {
    App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
}

fn push_tab(app: &mut App, service: Service) -> TabId {
    let id = TabId::new();
    app.tabs.push(Tab {
        id,
        service,
        title: service.name().to_string(),
        breadcrumb: service.name().to_string(),
    });
    app.current_tab = app.tabs.len() - 1;
    app.current_service = service;
    app.service_selected = true;
    app.mode = crate::keymap::Mode::Normal;
    id
}

#[test]
fn tab_ids_are_unique_stable_and_independent_of_index_and_title() {
    // Two tabs on the SAME service get distinct ids.
    let a = TabId::new();
    let b = TabId::new();
    assert_ne!(a, b, "every TabId::new() is unique even for identical tabs");

    let mut app = app();
    let first = push_tab(&mut app, Service::SnsTopics);
    let second = push_tab(&mut app, Service::SnsTopics);
    assert_ne!(first, second);

    // Identity survives reorder: swapping the Vec positions does not change ids.
    app.tabs.swap(0, 1);
    assert_eq!(app.tabs[0].id, second);
    assert_eq!(app.tabs[1].id, first);

    // Identity survives a title change.
    app.tabs[0].title = "renamed".into();
    assert_eq!(app.tabs[0].id, second);
}

#[test]
fn detached_fixture_is_usable_and_fabricates_no_id() {
    // No tabs at all: sync is a no-op and does not panic or invent an owner.
    let mut app = app();
    assert!(app.tabs.is_empty());
    sync_active(&mut app);
    assert!(app.tab_runtime.bound.is_none(), "unbound with no tabs");

    // A tab whose service disagrees with current_service (mid-transition) is not
    // adopted as an owner.
    push_tab(&mut app, Service::SnsTopics);
    app.current_service = Service::KmsKeys; // divergent
    sync_active(&mut app);
    assert!(app.tab_runtime.bound.is_none(), "divergent tab not bound");
}

#[test]
fn first_sync_adopts_live_state_without_replacing_prepared_fields() {
    let mut app = app();
    // Prepare fixture rows BEFORE the runtime binds.
    app.sns_state.topics.filter = "prepared".into();
    let owner = push_tab(&mut app, Service::SnsTopics);

    sync_active(&mut app);

    // Adopted: prepared fixture data is preserved, not reset to empty.
    assert_eq!(app.tab_runtime.bound, Some(owner));
    assert_eq!(app.sns_state.topics.filter, "prepared");
    assert!(app.tab_runtime.snapshots.is_empty());
}

#[test]
fn duplicate_same_service_tabs_keep_independent_rows() {
    let mut app = app();

    // Tab A: adopt with its own filter as a stand-in for independent view data.
    let a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    app.sns_state.topics.filter = "tab-a".into();

    // Open a second SNS tab. sync parks A and gives B a fresh (blank) state.
    let b = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    assert_eq!(app.tab_runtime.bound, Some(b));
    assert_eq!(
        app.sns_state.topics.filter, "",
        "new duplicate tab starts blank"
    );
    app.sns_state.topics.filter = "tab-b".into();

    // Switch back to A: its rows return; B is parked untouched.
    app.current_tab = app.tabs.iter().position(|t| t.id == a).unwrap();
    app.current_service = Service::SnsTopics;
    sync_active(&mut app);
    assert_eq!(app.tab_runtime.bound, Some(a));
    assert_eq!(app.sns_state.topics.filter, "tab-a");

    // And back to B.
    app.current_tab = app.tabs.iter().position(|t| t.id == b).unwrap();
    sync_active(&mut app);
    assert_eq!(app.sns_state.topics.filter, "tab-b");
}

#[test]
fn switching_tab_preserves_a_pending_read_on_the_parked_tab() {
    let mut app = app();

    // Tab A acquires a pending read (managed request on fresh open).
    let a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    // The live SNS state should report Loading after its first request.
    assert!(
        matches!(status_for(&app, a), ReadStatus::Loading { .. }),
        "fresh managed tab is Loading after its first request"
    );

    // Switch to a new tab B. A is parked.
    let b = push_tab(&mut app, Service::KmsKeys);
    sync_active(&mut app);

    // A's pending read is preserved in its snapshot, reported via status_for.
    assert!(
        matches!(status_for(&app, a), ReadStatus::Loading { .. }),
        "parked tab keeps its pending read"
    );
    // B is the visible owner now.
    assert_eq!(app.tab_runtime.bound, Some(b));
}

#[test]
fn closing_a_tab_aborts_only_its_own_read() {
    let mut app = app();
    let a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    let b = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);

    // Both A and B exist; A is parked and B is live.
    assert!(app.tab_runtime.snapshots.contains_key(&a));

    // Close the VISIBLE tab B.
    let b_index = app.tabs.iter().position(|t| t.id == b).unwrap();
    app.tabs.remove(b_index);
    purge_closed(&mut app);
    // Point at the remaining tab A and resync.
    app.current_tab = 0;
    app.current_service = Service::SnsTopics;
    sync_active(&mut app);

    // A's snapshot survives (it is now the live owner); B's is gone.
    assert!(
        !app.tab_runtime.snapshots.contains_key(&b),
        "closed tab purged"
    );
    assert_eq!(app.tab_runtime.bound, Some(a), "A is now the live owner");

    // Closing a PARKED tab drops only its snapshot, not the live owner's read.
    let c = push_tab(&mut app, Service::SnsTopics); // new parked-then-live
    sync_active(&mut app);
    // A is now parked. Close A (parked).
    let a_index = app.tabs.iter().position(|t| t.id == a).unwrap();
    app.tabs.remove(a_index);
    purge_closed(&mut app);
    assert!(!app.tab_runtime.snapshots.contains_key(&a));
    // C remains the live owner and is unaffected.
    app.current_tab = app.tabs.iter().position(|t| t.id == c).unwrap();
    app.current_service = Service::SnsTopics;
    sync_active(&mut app);
    assert_eq!(app.tab_runtime.bound, Some(c));
}

#[test]
fn owner_identity_tracks_across_reindexing() {
    let mut app = app();
    let a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    let b = push_tab(&mut app, Service::KmsKeys);
    sync_active(&mut app);
    let _c = push_tab(&mut app, Service::SqsQueues);
    sync_active(&mut app);

    // Remove the FIRST tab, shifting every later index down by one.
    app.tabs.remove(0);
    purge_closed(&mut app);
    // A's snapshot is purged; B and the live owner keep their identities.
    assert!(!app.tab_runtime.snapshots.contains_key(&a));

    // Re-point at B by SEARCHING for its id, not by a stale index, and confirm
    // the runtime binds the right owner.
    let b_index = app.tabs.iter().position(|t| t.id == b).unwrap();
    app.current_tab = b_index;
    app.current_service = Service::KmsKeys;
    sync_active(&mut app);
    assert_eq!(app.tab_runtime.bound, Some(b));
}

#[test]
fn session_replace_drops_all_snapshots_and_rebinds_fresh() {
    let mut app = app();
    let _a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    let _b = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    assert!(!app.tab_runtime.snapshots.is_empty());

    // Simulate a session load: brand-new tab list with brand-new ids.
    app.tabs.clear();
    let fresh = push_tab(&mut app, Service::SnsTopics);
    app.current_tab = 0;
    reset_for_session_replace(&mut app);
    sync_active(&mut app);

    assert!(
        app.tab_runtime.snapshots.is_empty(),
        "old snapshots dropped"
    );
    assert_eq!(app.tab_runtime.bound, Some(fresh));
    // Fresh managed tab starts blank and loading (requested once).
    assert_eq!(app.sns_state.topics.filter, "");
}

#[test]
fn cli_output_tabs_are_never_snapshotted() {
    let mut app = app();
    // A CLI output tab must not become a runtime owner or get a snapshot.
    app.tabs.push(Tab {
        id: TabId::new(),
        service: Service::CliOutput(1),
        title: "CLI".into(),
        breadcrumb: "CLI".into(),
    });
    app.current_tab = 0;
    app.current_service = Service::CliOutput(1);
    sync_active(&mut app);
    assert_eq!(app.tab_runtime.bound, Some(app.tabs[0].id));
    assert!(app.tab_runtime.snapshots.is_empty());
}

fn topic(name: &str) -> rusticity_core::SnsTopic {
    rusticity_core::SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}"))
        .unwrap()
}

fn start_local_topics(
    app: &mut App,
) -> tokio::sync::oneshot::Sender<Result<Vec<rusticity_core::SnsTopic>, String>> {
    crate::sns::inventory::request(app);
    let (tx, rx) = tokio::sync::oneshot::channel();
    assert!(app
        .sns_state
        .inventory
        .start(async move { rx.await.unwrap() }));
    tx
}

fn screen(app: &App) -> String {
    use ratatui::{backend::TestBackend, Terminal};
    let mut terminal = Terminal::new(TestBackend::new(240, 35)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(frame, app))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[tokio::test]
async fn duplicate_tabs_finish_independently_without_stealing_focus_or_cancelling() {
    let mut app = app();
    let a = push_tab(&mut app, Service::SnsTopics);
    app.tabs[0].title = "First SNS".into();
    sync_active(&mut app);
    let tx_a = start_local_topics(&mut app);
    let b = push_tab(&mut app, Service::SnsTopics);
    app.tabs[1].title = "Second SNS".into();
    sync_active(&mut app);
    let tx_b = start_local_topics(&mut app);
    assert!(has_background_work(&app));
    assert!(matches!(status_for(&app, a), ReadStatus::Loading { .. }));
    assert!(matches!(status_for(&app, b), ReadStatus::Loading { .. }));

    tx_a.send(Ok(vec![topic("first-result")])).unwrap();
    tokio::task::yield_now().await;
    assert!(poll(&mut app).await);
    assert_eq!(app.current_tab, 1);
    assert_eq!(app.current_service, Service::SnsTopics);
    assert_eq!(status_for(&app, a), ReadStatus::Ready);
    assert!(matches!(status_for(&app, b), ReadStatus::Loading { .. }));
    assert!(app.sns_state.topics.items.is_empty());
    let rendered = screen(&app);
    assert!(rendered.contains("First SNS ✓"));
    assert!(rendered.contains("Loading SNS topics"));
    assert!(!rendered.contains("first-result"));

    app.handle_action(crate::keymap::Action::PrevTab);
    assert_eq!(app.tabs[app.current_tab].id, a);
    assert_eq!(app.sns_state.topics.items[0].name, "first-result");
    assert!(!app.sns_state.inventory.is_busy());
    app.handle_action(crate::keymap::Action::Refresh);
    assert!(app.sns_state.topics.items.is_empty());
    assert!(!screen(&app).contains("first-result"));
    app.handle_action(crate::keymap::Action::GoBack); // cancel this read only
    assert!(!app.sns_state.inventory.is_busy());
    assert!(screen(&app).contains("Cancelled"));
    assert!(matches!(status_for(&app, b), ReadStatus::Loading { .. }));

    tx_b.send(Ok(vec![])).unwrap();
    tokio::task::yield_now().await;
    assert!(poll(&mut app).await);
    assert_eq!(app.tabs[app.current_tab].id, a);
    assert_eq!(status_for(&app, b), ReadStatus::Ready); // empty success is loaded
    assert!(screen(&app).contains("Second SNS ✓"));
    app.handle_action(crate::keymap::Action::NextTab);
    assert_eq!(app.tabs[app.current_tab].id, b);
    assert_eq!(app.sns_state.inventory.status(), ReadStatus::Ready);
    assert!(app.sns_state.topics.items.is_empty());
    assert!(!has_background_work(&app));
}

#[tokio::test]
async fn background_failure_marks_only_its_tab_and_never_shows_old_rows() {
    let mut app = app();
    let a = push_tab(&mut app, Service::SnsTopics);
    app.tabs[0].title = "Failed SNS".into();
    sync_active(&mut app);
    app.sns_state.topics.items = vec![topic("OLD-ROW-MUST-DISAPPEAR")];
    let tx = start_local_topics(&mut app);
    assert!(app.sns_state.topics.items.is_empty());
    let b = push_tab(&mut app, Service::VpcVpcs);
    sync_active(&mut app);
    tx.send(Err("AccessDenied: fixture".into())).unwrap();
    tokio::task::yield_now().await;
    assert!(poll(&mut app).await);
    assert_eq!(app.tabs[app.current_tab].id, b);
    assert_eq!(status_for(&app, a), ReadStatus::Failed);
    assert!(screen(&app).contains("Failed SNS !"));
    app.handle_action(crate::keymap::Action::PrevTab);
    let text = screen(&app);
    assert!(text.contains("AccessDenied") && text.contains("Ctrl+R"));
    assert!(!text.contains("OLD-ROW-MUST-DISAPPEAR"));
}

#[tokio::test]
async fn closing_visible_owner_aborts_only_that_job_and_reindex_keeps_other_result() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let mut app = app();
    let a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    let tx = start_local_topics(&mut app);
    let b = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    let stopped = Arc::new(AtomicBool::new(false));
    let guard = Guard(stopped.clone());
    assert!(app.sns_state.inventory.start(async move {
        let _guard = guard;
        std::future::pending().await
    }));
    tokio::task::yield_now().await;
    app.handle_action(crate::keymap::Action::CloseTab);
    tokio::task::yield_now().await;
    assert!(stopped.load(Ordering::SeqCst));
    assert!(app.tabs.iter().all(|tab| tab.id != b));
    assert_eq!(app.tabs[app.current_tab].id, a);
    tx.send(Ok(vec![topic("surviving-owner")])).unwrap();
    tokio::task::yield_now().await;
    assert!(poll(&mut app).await);
    assert_eq!(app.sns_state.topics.items[0].name, "surviving-owner");
}

#[tokio::test]
async fn aws_context_change_clears_ready_background_data_before_next_draw() {
    let mut app = app();
    let a = push_tab(&mut app, Service::SnsTopics);
    sync_active(&mut app);
    let tx = start_local_topics(&mut app);
    tx.send(Ok(vec![topic("OLD-ACCOUNT-ROW")])).unwrap();
    tokio::task::yield_now().await;
    poll(&mut app).await;
    let b = push_tab(&mut app, Service::VpcVpcs);
    sync_active(&mut app);
    assert_eq!(status_for(&app, a), ReadStatus::Ready);
    app.config.account_id = "999999999999".into();
    sync_active(&mut app);
    assert_eq!(app.tabs[app.current_tab].id, b);
    assert_eq!(status_for(&app, a), ReadStatus::Idle);
    app.handle_action(crate::keymap::Action::PrevTab);
    assert!(app.sns_state.topics.items.is_empty());
    assert!(!screen(&app).contains("OLD-ACCOUNT-ROW"));
    assert!(matches!(status_for(&app, a), ReadStatus::Loading { .. }));
}

#[test]
fn legacy_duplicate_tabs_load_once_and_return_without_automatic_reload() {
    let mut app = app();
    let before = app.tabs.clone();
    let a = push_tab(&mut app, Service::VpcVpcs);
    sync_active(&mut app);
    assert!(opened_new_tab(&app, &before));
    app.vpc_state.vpcs.items = vec![rusticity_core::vpc::Vpc {
        vpc_id: "vpc-a".into(),
        ..Default::default()
    }];
    let before = app.tabs.clone();
    let b = push_tab(&mut app, Service::VpcVpcs);
    sync_active(&mut app);
    assert!(
        opened_new_tab(&app, &before),
        "same service is still a NEW tab"
    );
    assert!(!restored_existing_tab(&app, &before, 0));
    assert!(app.vpc_state.vpcs.items.is_empty());
    app.vpc_state.vpcs.items = vec![rusticity_core::vpc::Vpc {
        vpc_id: "vpc-b".into(),
        ..Default::default()
    }];
    let before = app.tabs.clone();
    app.handle_action(crate::keymap::Action::PrevTab);
    assert_eq!(app.tabs[app.current_tab].id, a);
    assert!(restored_existing_tab(&app, &before, 1));
    assert!(!opened_new_tab(&app, &before));
    assert_eq!(app.vpc_state.vpcs.items[0].vpc_id, "vpc-a");
    app.handle_action(crate::keymap::Action::NextTab);
    assert_eq!(app.tabs[app.current_tab].id, b);
    assert_eq!(app.vpc_state.vpcs.items[0].vpc_id, "vpc-b");
    let before = app.tabs.clone();
    app.handle_action(crate::keymap::Action::CloseTab);
    assert!(restored_existing_tab(&app, &before, 1));
    assert_eq!(app.vpc_state.vpcs.items[0].vpc_id, "vpc-a");
}
