use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn scope(resource: &str) -> ReadScope {
    ReadScope {
        owner: None,
        profile: "test".into(),
        region: "us-east-1".into(),
        account: "123456789012".into(),
        role: "role".into(),
        resource: resource.into(),
    }
}

#[tokio::test]
async fn queue_does_not_execute_and_unfinished_poll_does_not_wait() {
    let key = scope("resource");
    let mut read = AsyncRead::<String>::requested(key.clone());
    assert!(read.is_pending() && read.is_busy() && !read.is_running());
    assert!(read.take_ready(&key).await.is_none());
    let started = Arc::new(AtomicBool::new(false));
    let flag = started.clone();
    assert!(read.start(async move {
        flag.store(true, Ordering::SeqCst);
        std::future::pending().await
    }));
    assert!(!read.is_pending() && read.is_running());
    tokio::task::yield_now().await;
    assert!(started.load(Ordering::SeqCst));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), read.take_ready(&key))
            .await
            .unwrap()
            .is_none()
    );
    read.cancel();
    assert!(!read.is_busy());
    assert!(read.matches_scope(&key));
}

#[tokio::test]
async fn consumes_each_success_once_and_retains_scope_for_cached_data() {
    let key = scope("resource");
    let mut read = AsyncRead::requested(key.clone());
    assert!(read.start(async { Ok(42) }));
    tokio::task::yield_now().await;
    assert_eq!(read.take_ready(&key).await.unwrap().unwrap(), 42);
    assert!(read.take_ready(&key).await.is_none());
    assert!(!read.is_busy());
    assert!(read.matches_scope(&key));
    assert!(read.invalidate_if_changed(&scope("different resource")));
    assert!(!read.matches_scope(&key));
}

#[tokio::test]
async fn service_errors_and_task_panics_remain_distinct() {
    let key = scope("resource");
    let mut read = AsyncRead::<()>::requested(key.clone());
    read.start(async { Err("AccessDenied: metadata".into()) });
    tokio::task::yield_now().await;
    assert_eq!(
        read.take_ready(&key)
            .await
            .unwrap()
            .unwrap_err()
            .into_message("read failed"),
        "AccessDenied: metadata"
    );
    read.request(key.clone());
    read.start(async {
        panic!("test task panic");
    });
    tokio::task::yield_now().await;
    let err = read.take_ready(&key).await.unwrap().unwrap_err();
    assert!(matches!(err, ReadError::Task(_)));
    assert!(err.into_message("read failed").starts_with("read failed: "));
    assert!(!read.is_busy());
}

#[tokio::test]
async fn refresh_discards_even_a_completed_old_result_for_the_same_scope() {
    let key = scope("resource");
    let mut read = AsyncRead::requested(key.clone());
    read.start(async { Ok("old") });
    tokio::task::yield_now().await;
    read.request(key.clone());
    assert!(read.take_ready(&key).await.is_none());
    read.start(async { Ok("new") });
    tokio::task::yield_now().await;
    assert_eq!(read.take_ready(&key).await.unwrap().unwrap(), "new");
}

struct Dropped(Arc<AtomicBool>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn replacing_resetting_and_dropping_abort_inflight_work() {
    for action in 0..3 {
        let dropped = Arc::new(AtomicBool::new(false));
        let guard = Dropped(dropped.clone());
        let mut read = AsyncRead::<()>::requested(scope("resource"));
        read.start(async move {
            let _guard = guard;
            std::future::pending().await
        });
        tokio::task::yield_now().await;
        match action {
            0 => read.request(scope("new resource")),
            1 => read.reset(),
            _ => drop(read),
        }
        tokio::task::yield_now().await;
        assert!(dropped.load(Ordering::SeqCst));
    }
}

#[tokio::test]
async fn every_identity_component_invalidates_ready_or_queued_work() {
    let original = scope("resource");
    for change in 0..5 {
        let mut current = original.clone();
        match change {
            0 => current.profile = "other".into(),
            1 => current.region = "eu-west-1".into(),
            2 => current.account = "999999999999".into(),
            3 => current.role = "other role".into(),
            _ => current.resource = "other resource".into(),
        }
        for completed in [false, true] {
            let mut read = AsyncRead::requested(original.clone());
            if completed {
                read.start(async { Ok("obsolete") });
                tokio::task::yield_now().await;
            }
            assert!(read.take_ready(&current).await.is_none());
            assert!(!read.is_busy());
            assert!(!read.matches_scope(&original));
        }
    }
}

#[tokio::test]
async fn start_without_a_request_never_spawns_a_future() {
    let mut read = AsyncRead::<()>::default();
    assert!(!read.start(async {
        panic!("must never run");
    }));
    assert!(!read.is_busy());
    assert!(!read.invalidate_if_changed(&scope("resource")));
}

#[test]
fn app_scope_captures_identity_and_explicit_resource_not_row_selection() {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    app.config.role_arn = "role".into();
    assert_eq!(ReadScope::from_app(&app, "resource"), scope("resource"));
    app.sns_state.topics.selected = usize::MAX;
    assert_eq!(ReadScope::from_app(&app, "resource"), scope("resource"));
}
