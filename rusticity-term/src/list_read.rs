//! One tab-owned root inventory read: the shared request, cancel, poll and
//! status policy for list views. `request` clears the rows at once and only
//! queues; `drive` starts the queued read and accepts a result only while the
//! owner's AWS context and resource are unchanged. A failed, cancelled or
//! superseded read never leaves older rows on screen.
use crate::async_read::{AsyncRead, ReadScope, ReadStatus};
use crate::table::TableState;
use std::future::Future;

pub(crate) struct ListRead<T> {
    read: AsyncRead<Vec<T>>,
    /// Inline error from the latest read; never an ErrorModal focus steal.
    error: Option<String>,
}

impl<T> Default for ListRead<T> {
    fn default() -> Self {
        Self {
            read: AsyncRead::default(),
            error: None,
        }
    }
}

fn clean(message: String) -> String {
    message
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

impl<T: Send + 'static> ListRead<T> {
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Only a fresh successful read is Ready; nothing loaded yet stays Idle.
    pub(crate) fn status(&self) -> ReadStatus {
        match self.read.status() {
            ReadStatus::Ready | ReadStatus::Idle if self.error.is_some() => ReadStatus::Failed,
            status => status,
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.read.is_busy()
    }

    /// Input-handler safe: clears rows at once and queues the read.
    pub(crate) fn request(&mut self, table: &mut TableState<T>, scope: ReadScope) {
        table.items.clear();
        table.reset();
        self.error = None;
        self.read.request(scope);
    }

    /// Tests: treat rows placed directly in the table as a fresh success.
    #[cfg(test)]
    pub(crate) fn mark_loaded(&mut self, scope: ReadScope) {
        self.error = None;
        self.read.mark_loaded(scope);
    }

    /// Esc while loading: abort the read and say so.
    pub(crate) fn cancel(&mut self, table: &mut TableState<T>) {
        self.read.cancel();
        table.items.clear();
        table.reset();
        self.error = Some("Cancelled — Ctrl+R to retry".into());
    }

    /// Replace the rows with an inline error after a successful read, for a
    /// result that cannot be used (an opened resource that is gone).
    pub(crate) fn fail(&mut self, table: &mut TableState<T>, message: String) {
        table.items.clear();
        table.reset();
        self.error = Some(clean(message));
    }

    /// Start a queued read and harvest a finished one. Returns true when the
    /// rows or status changed.
    pub(crate) async fn drive<F>(
        &mut self,
        table: &mut TableState<T>,
        scope: &ReadScope,
        what: &str,
        start: impl FnOnce() -> F,
    ) -> bool
    where
        F: Future<Output = Result<Vec<T>, String>> + Send + 'static,
    {
        // A profile/Region/account/role change drops the rows with no late repaint.
        if self.read.invalidate_if_changed(scope) {
            table.items.clear();
            table.reset();
            self.error = None;
            return true;
        }
        let mut changed = false;
        if self.read.is_pending() {
            changed = self.read.start(start());
        }
        if let Some(result) = self.read.take_ready(scope).await {
            table.items.clear();
            table.reset();
            match result {
                Ok(rows) => {
                    table.items = rows;
                    self.error = None;
                }
                Err(error) => self.error = Some(clean(error.into_message(what))),
            }
            changed = true;
        }
        changed
    }
}
