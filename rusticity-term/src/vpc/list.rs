//! The shared parts of the read-only VPC lists with no detail view (Subnets,
//! Security groups): one spec table filled by one tab-owned read, bounded by
//! 30s. Enter expands a row. See `crate::list_read` for the freshness rules.
use crate::{
    app::App,
    async_read::{ReadScope, ReadStatus},
    keymap::{Action, Mode},
    list_read::ListRead,
    spec_table::{Spec, Table},
};
use ratatui::style::{Color, Style};
use std::{future::Future, time::Duration};

const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// One list with its read.
pub struct State<T: 'static> {
    pub(crate) list: Table<T>,
    pub(crate) inventory: ListRead<T>,
}

impl<T: Send + 'static> State<T> {
    pub(crate) fn new(list: Table<T>) -> Self {
        Self {
            list,
            inventory: ListRead::default(),
        }
    }

    pub(crate) fn load_status(&self) -> ReadStatus {
        self.inventory.status()
    }

    pub(crate) fn request(&mut self, scope: ReadScope) {
        self.inventory.request(&mut self.list.rows, scope);
    }

    /// The selected row on screen.
    pub(crate) fn selected(&self) -> Option<&T> {
        self.list.filtered().get(self.list.rows.selected).copied()
    }

    /// Esc while loading cancels; otherwise Esc is not ours.
    fn cancel(&mut self) -> bool {
        if !self.inventory.is_busy() {
            return false;
        }
        self.inventory.cancel(&mut self.list.rows);
        true
    }

    /// Start a queued read and take a finished one, bounded by 30s.
    pub(crate) async fn drive<F>(&mut self, scope: &ReadScope, what: &str, read: F) -> bool
    where
        F: Future<Output = anyhow::Result<Vec<T>>> + Send + 'static,
    {
        let timeout = format!("{what} timed out after {READ_TIMEOUT:?}");
        let failed = format!("{what} failed");
        self.inventory
            .drive(&mut self.list.rows, scope, &failed, || async move {
                match tokio::time::timeout(READ_TIMEOUT, read).await {
                    Ok(result) => result.map_err(|e| format!("{e:#}")),
                    Err(_) => Err(timeout),
                }
            })
            .await
    }
}

/// A list that opens on the given columns, ordered by `column` ascending.
pub(crate) fn table<T>(spec: Spec<T>, visible: &[&'static str], column: &'static str) -> Table<T> {
    Table::new(spec)
        .showing(visible)
        .sorted_by(crate::sort::SortOrder::new(
            column,
            crate::common::SortDirection::Asc,
        ))
}

/// How one list service reaches its state and its read.
pub(crate) struct Service<T: 'static> {
    pub(crate) state: fn(&mut App) -> &mut State<T>,
    pub(crate) request: fn(&mut App),
    /// Filter, paging and row keys on the list.
    pub(crate) input: fn(&mut App, &Action) -> bool,
}

/// Ctrl+R, Esc and the error modal, then the shared table keys. The caller
/// checks that its service is the one on screen.
pub(crate) fn handle_action<T: Send + 'static>(
    app: &mut App,
    service: Service<T>,
    action: &Action,
) -> bool {
    match app.mode {
        Mode::ErrorModal => {
            if crate::elb::retry_from_error(app, action) {
                (service.request)(app);
                return true;
            }
            false
        }
        Mode::Normal => match action {
            Action::Refresh => {
                (service.request)(app);
                true
            }
            Action::GoBack => (service.state)(app).cancel(),
            Action::OpenColumnSelector => false,
            _ if crate::elb::global(action) => false,
            _ => (service.input)(app, action),
        },
        Mode::FilterInput => (service.input)(app, action),
        _ => false,
    }
}

pub(crate) fn yes_no(value: Option<bool>) -> String {
    match value {
        Some(true) => "Yes".into(),
        Some(false) => "No".into(),
        None => String::new(),
    }
}

/// Colors of the shared VPC labels: see `crate::vpc::state_label`.
pub(crate) fn style(text: &str) -> Style {
    let color = if text.starts_with('✅') {
        Some(Color::Green)
    } else if text.starts_with('⏳') || text == "Unavailable" {
        Some(Color::Yellow)
    } else if text.starts_with("failed") || text == "unavailable" {
        Some(Color::Red)
    } else {
        None
    };
    color.map_or_else(Style::default, |c| Style::default().fg(c))
}
