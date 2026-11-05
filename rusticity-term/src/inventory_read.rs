//! A tab-owned background read for a root inventory table, with the display
//! policy every such table shares: rows are cleared the moment a read starts,
//! a failed or cancelled read shows its error with no older rows, and a result
//! is accepted only for the AWS context it was requested in.
//!
//! The owning service supplies the scope, the fetch future and any extra
//! teardown (such as closing an entered detail on a context change).
use crate::async_read::{AsyncRead, ReadScope, ReadStatus};
use crate::table::TableState;
use std::future::Future;
use std::time::Duration;

pub(crate) const CANCELLED: &str = "Cancelled — Ctrl+R to retry";

pub(crate) struct OwnedInventory<T> {
    read: AsyncRead<Vec<T>>,
    error: Option<String>,
}

impl<T> Default for OwnedInventory<T> {
    fn default() -> Self {
        Self {
            read: AsyncRead::default(),
            error: None,
        }
    }
}

/// A cloned service state never inherits another owner's in-flight read.
impl<T> Clone for OwnedInventory<T> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<T> std::fmt::Debug for OwnedInventory<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedInventory")
            .field("status", &self.read.status())
            .field("error", &self.error)
            .finish()
    }
}

impl<T: Send + 'static> OwnedInventory<T> {
    /// Clear rows and queue a read. Input-handler safe: no request is sent here.
    pub(crate) fn request(&mut self, table: &mut TableState<T>, scope: ReadScope) {
        clear(table);
        self.error = None;
        self.read.request(scope);
    }

    /// Esc while a read is busy. Returns false when there is nothing to cancel.
    pub(crate) fn cancel(&mut self, table: &mut TableState<T>) -> bool {
        if !self.read.is_busy() {
            return false;
        }
        self.read.cancel();
        clear(table);
        self.error = Some(CANCELLED.into());
        true
    }

    /// Rows came from an in-line read for `scope` (finder or bookmark open).
    pub(crate) fn mark_loaded(&mut self, scope: ReadScope) {
        self.error = None;
        self.read.mark_loaded(scope);
    }

    /// Drive one step. `fetch` is called only when a read is queued. Returns
    /// `Some(changed)`, or `None` when the AWS context changed and the rows
    /// were dropped (the caller may close an entered detail as well).
    pub(crate) async fn poll<F, Fut>(
        &mut self,
        table: &mut TableState<T>,
        scope: &ReadScope,
        label: &str,
        timeout: Duration,
        fetch: F,
    ) -> Option<bool>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<Vec<T>>> + Send + 'static,
    {
        if self.read.invalidate_if_changed(scope) {
            clear(table);
            self.error = None;
            return None;
        }
        let mut changed = false;
        if self.read.is_pending() {
            let fut = fetch();
            let what = label.to_string();
            changed = self.read.start(async move {
                match tokio::time::timeout(timeout, fut).await {
                    Ok(result) => result.map_err(|e| format!("{e:#}")),
                    Err(_) => Err(format!(
                        "{what} read timed out after {}s",
                        timeout.as_secs()
                    )),
                }
            });
        }
        if let Some(result) = self.read.take_ready(scope).await {
            clear(table);
            match result {
                Ok(rows) => {
                    table.items = rows;
                    self.error = None;
                }
                Err(error) => {
                    self.error = Some(sanitize(
                        error.into_message(&format!("{label} read failed")),
                    ))
                }
            }
            changed = true;
        }
        Some(changed)
    }
}

impl<T> OwnedInventory<T> {
    /// Fresh-only status. Seeded rows with no read (tests, legacy fixtures)
    /// count as loaded; an empty table with no read stays "Not loaded".
    pub(crate) fn status(&self, table: &TableState<T>) -> ReadStatus {
        match self.read.status() {
            _ if self.error.is_some() && !self.read.is_busy() => ReadStatus::Failed,
            ReadStatus::Idle if !table.items.is_empty() => ReadStatus::Ready,
            status => status,
        }
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn start_for_test(
        &mut self,
        read: impl Future<Output = Result<Vec<T>, String>> + Send + 'static,
    ) -> bool
    where
        T: Send + 'static,
    {
        self.read.start(read)
    }
}

fn clear<T>(table: &mut TableState<T>) {
    table.items.clear();
    table.reset();
}

fn sanitize(message: String) -> String {
    message
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[cfg(test)]
mod service_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    fn scope(app: &App) -> ReadScope {
        ReadScope::from_app(app, "inventory:test")
    }

    async fn settle(
        inv: &mut OwnedInventory<u32>,
        table: &mut TableState<u32>,
        scope: &ReadScope,
    ) -> Option<bool> {
        tokio::task::yield_now().await;
        inv.poll(table, scope, "Test", Duration::from_secs(5), || async {
            unreachable!("already started")
        })
        .await
    }

    #[tokio::test]
    async fn request_clears_rows_and_failure_never_restores_them() {
        let app = App::new_without_client("t".into(), Some("us-east-1".into()));
        let scope = scope(&app);
        let mut table = TableState::new();
        table.items = vec![1, 2];
        let mut inv = OwnedInventory::default();
        inv.request(&mut table, scope.clone());
        assert!(table.items.is_empty());
        assert!(matches!(inv.status(&table), ReadStatus::Loading { .. }));
        assert!(inv.start_for_test(async { Err("AccessDenied".into()) }));
        assert_eq!(settle(&mut inv, &mut table, &scope).await, Some(true));
        assert_eq!(inv.status(&table), ReadStatus::Failed);
        assert_eq!(inv.error(), Some("AccessDenied"));
        assert!(table.items.is_empty());
    }

    #[tokio::test]
    async fn success_replaces_rows_and_context_change_drops_them() {
        let mut app = App::new_without_client("t".into(), Some("us-east-1".into()));
        let scope0 = scope(&app);
        let mut table = TableState::new();
        let mut inv = OwnedInventory::default();
        inv.request(&mut table, scope0.clone());
        let changed = inv
            .poll(
                &mut table,
                &scope0,
                "Test",
                Duration::from_secs(5),
                || async { Ok(vec![7, 8]) },
            )
            .await;
        assert_eq!(changed, Some(true), "the read started");
        assert_eq!(settle(&mut inv, &mut table, &scope0).await, Some(true));
        assert_eq!(table.items, [7, 8]);
        assert_eq!(inv.status(&table), ReadStatus::Ready);
        app.config.region = "eu-west-1".into();
        assert_eq!(settle(&mut inv, &mut table, &scope(&app)).await, None);
        assert!(table.items.is_empty());
    }

    #[tokio::test]
    async fn timeout_is_an_error_and_cancel_shows_retry_hint() {
        let app = App::new_without_client("t".into(), Some("us-east-1".into()));
        let scope = scope(&app);
        let mut table = TableState::new();
        let mut inv = OwnedInventory::<u32>::default();
        inv.request(&mut table, scope.clone());
        inv.poll(
            &mut table,
            &scope,
            "Slow",
            Duration::from_millis(1),
            || async { std::future::pending::<anyhow::Result<Vec<u32>>>().await },
        )
        .await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(settle(&mut inv, &mut table, &scope).await, Some(true));
        assert!(inv.error().unwrap().contains("Slow read timed out"));

        inv.request(&mut table, scope.clone());
        assert!(inv.cancel(&mut table));
        assert_eq!(inv.error(), Some(CANCELLED));
        assert_eq!(inv.status(&table), ReadStatus::Failed);
        assert!(!inv.cancel(&mut table), "nothing left to cancel");
    }

    #[tokio::test]
    async fn mark_loaded_drops_the_queued_read_so_inline_rows_survive() {
        let app = App::new_without_client("t".into(), Some("us-east-1".into()));
        let scope = scope(&app);
        let mut table = TableState::new();
        let mut inv = OwnedInventory::default();
        inv.request(&mut table, scope.clone());
        table.items = vec![42];
        inv.mark_loaded(scope.clone());
        assert_eq!(inv.status(&table), ReadStatus::Ready);
        let changed = inv
            .poll(
                &mut table,
                &scope,
                "Test",
                Duration::from_secs(5),
                || async { Ok(vec![1, 2, 3]) },
            )
            .await;
        assert_eq!(changed, Some(false));
        assert_eq!(table.items, [42]);
    }

    #[test]
    fn seeded_rows_without_a_read_count_as_loaded() {
        let mut table = TableState::new();
        let inv = OwnedInventory::<u32>::default();
        assert_eq!(inv.status(&table), ReadStatus::Idle);
        table.items = vec![1];
        assert_eq!(inv.status(&table), ReadStatus::Ready);
    }
}
