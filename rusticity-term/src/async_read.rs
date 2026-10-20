//! Shared ownership of finite background reads. Views own presentation policy:
//! this module owns queueing, abort-on-drop and accepting only current results.
use crate::app::App;
use std::{future::Future, time::Instant};

/// View-facing state of the most recent request, never inferred from row count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ReadStatus {
    #[default]
    Idle,
    Loading {
        started: Instant,
    },
    Ready,
    Failed,
}
use tokio::task::{JoinError, JoinHandle};

/// AWS identity plus the entered resource, independent of the highlighted row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadScope {
    owner: Option<crate::tab_runtime::TabId>,
    profile: String,
    region: String,
    account: String,
    role: String,
    resource: String,
}
impl ReadScope {
    /// Background completions compare against their owner, not current focus.
    pub(crate) fn with_owner(mut self, owner: Option<crate::tab_runtime::TabId>) -> Self {
        self.owner = owner;
        self
    }
    pub(crate) fn from_app(app: &App, resource: impl Into<String>) -> Self {
        Self {
            owner: app
                .tabs
                .get(app.current_tab)
                .filter(|tab| tab.service == app.current_service)
                .map(|tab| tab.id),
            profile: app.profile.clone(),
            region: app.config.region.clone(),
            account: app.config.account_id.clone(),
            role: app.config.role_arn.clone(),
            resource: resource.into(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ReadError {
    Service(String),
    Task(JoinError),
}
impl ReadError {
    /// Preserve a service error verbatim; callers retain their existing wording
    /// for a crashed/cancelled task and their own display sanitization.
    pub(crate) fn into_message(self, task_prefix: &str) -> String {
        match self {
            Self::Service(message) => message,
            Self::Task(error) => format!("{task_prefix}: {error}"),
        }
    }
}

struct Running<T>(JoinHandle<Result<T, String>>);
impl<T> Drop for Running<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) struct AsyncRead<T> {
    // Retained after completion so cached data can still be invalidated.
    scope: Option<ReadScope>,
    pending: bool,
    status: ReadStatus,
    running: Option<Running<T>>,
}
impl<T> Default for AsyncRead<T> {
    fn default() -> Self {
        Self {
            scope: None,
            pending: false,
            status: ReadStatus::Idle,
            running: None,
        }
    }
}
impl<T> AsyncRead<T> {
    /// Completed local fixture; production reads must go through start/take_ready.
    #[cfg(test)]
    pub(crate) fn resolved(scope: ReadScope, success: bool) -> Self {
        Self {
            scope: Some(scope),
            pending: false,
            running: None,
            status: if success {
                ReadStatus::Ready
            } else {
                ReadStatus::Failed
            },
        }
    }
    pub(crate) fn requested(scope: ReadScope) -> Self {
        Self {
            scope: Some(scope),
            pending: true,
            status: ReadStatus::Loading {
                started: Instant::now(),
            },
            running: None,
        }
    }
    /// Supersede even a finished-but-not-consumed read. This intentionally does
    /// not touch displayed data: retaining or clearing it is a view policy.
    pub(crate) fn request(&mut self, scope: ReadScope) {
        self.running = None;
        self.scope = Some(scope);
        self.pending = true;
        self.status = ReadStatus::Loading {
            started: Instant::now(),
        };
    }
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
    /// Cancel work while retaining the scope of any cached view data.
    pub(crate) fn cancel(&mut self) {
        self.running = None;
        self.pending = false;
        self.status = ReadStatus::Idle;
    }
    pub(crate) fn status(&self) -> ReadStatus {
        self.status
    }
    pub(crate) fn matches_scope(&self, scope: &ReadScope) -> bool {
        self.scope.as_ref() == Some(scope)
    }
    pub(crate) fn invalidate_if_changed(&mut self, scope: &ReadScope) -> bool {
        if self.scope.as_ref().is_some_and(|saved| saved != scope) {
            self.reset();
            true
        } else {
            false
        }
    }
    pub(crate) fn is_pending(&self) -> bool {
        self.pending
    }
    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running.is_some()
    }
    pub(crate) fn is_busy(&self) -> bool {
        self.pending || self.running.is_some()
    }

    /// Never wait on an unfinished task in the terminal event loop. The scope
    /// check also protects callers that forgot to invalidate before polling.
    pub(crate) async fn take_ready(&mut self, scope: &ReadScope) -> Option<Result<T, ReadError>> {
        if self.invalidate_if_changed(scope) {
            return None;
        }
        if !self
            .running
            .as_ref()
            .is_some_and(|task| task.0.is_finished())
        {
            return None;
        }
        let mut task = self.running.take().unwrap();
        let result = match (&mut task.0).await {
            Ok(result) => result.map_err(ReadError::Service),
            Err(error) => Err(ReadError::Task(error)),
        };
        self.status = if result.is_ok() {
            ReadStatus::Ready
        } else {
            ReadStatus::Failed
        };
        Some(result)
    }
}
impl<T: Send + 'static> AsyncRead<T> {
    /// Called only by a view's async poll, never by its input handler. Returns
    /// false when no request is queued; the supplied future is then dropped.
    pub(crate) fn start(
        &mut self,
        read: impl Future<Output = Result<T, String>> + Send + 'static,
    ) -> bool {
        if !self.pending {
            return false;
        }
        self.pending = false;
        self.running = Some(Running(tokio::spawn(read)));
        true
    }
}

#[cfg(test)]
mod tests;
