//! Elastic Load Balancing under EC2: Load balancers and Target groups, each a
//! tab-owned list that opens a detail page with the console tabs.
//!
//! Each level owns one cancelable read scoped to the tab, the AWS context and
//! the resource. Entering, refreshing or leaving a level clears it at once;
//! nothing older is shown during a reload or after a failed one. Read-only.
use crate::{
    app::App,
    async_read::{AsyncRead, ReadScope, ReadStatus},
    keymap::Action,
    spec_table::Spec,
};
use ratatui::style::{Color, Style};
use std::future::Future;

pub mod load_balancers;
pub mod target_groups;
mod view;
pub use view::{render_load_balancers, render_target_groups};

#[derive(Clone, Debug)]
pub struct TagRow {
    pub key: String,
    pub value: String,
}

pub(crate) const TAG_COLUMNS: Spec<TagRow> = &[
    ("Key", 32, |t| t.key.clone()),
    ("Value", 60, |t| t.value.clone()),
];

pub(crate) fn tag_rows(tags: &[(String, String)]) -> Vec<TagRow> {
    tags.iter()
        .map(|(key, value)| TagRow {
            key: key.clone(),
            value: value.clone(),
        })
        .collect()
}

/// The fresh detail of the entered resource, its read and its error.
pub(crate) struct DetailRead<D> {
    detail: Option<Box<D>>,
    error: Option<String>,
    read: AsyncRead<Box<D>>,
}

impl<D: Send + 'static> DetailRead<D> {
    pub(crate) fn requested(scope: ReadScope) -> Self {
        Self {
            detail: None,
            error: None,
            read: AsyncRead::requested(scope),
        }
    }

    pub(crate) fn detail(&self) -> Option<&D> {
        self.detail.as_deref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn status(&self) -> ReadStatus {
        if self.read.is_busy() {
            self.read.status()
        } else if self.error.is_some() {
            ReadStatus::Failed
        } else if self.detail.is_some() {
            ReadStatus::Ready
        } else {
            ReadStatus::Idle
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.read.is_busy()
    }

    pub(crate) fn matches_scope(&self, scope: &ReadScope) -> bool {
        self.read.matches_scope(scope)
    }

    /// Drop the shown detail at once and queue a fresh read.
    pub(crate) fn request(&mut self, scope: ReadScope) {
        self.detail = None;
        self.error = None;
        self.read.request(scope);
    }

    /// Esc while loading: abort and say so.
    pub(crate) fn cancel(&mut self) {
        self.read.cancel();
        self.detail = None;
        self.error = Some("Cancelled — Ctrl+R to retry".into());
    }

    /// Start a queued read and take a finished one. Returns true when a
    /// result arrived: the caller refills its tables from `detail()`.
    pub(crate) async fn drive<F>(
        &mut self,
        scope: &ReadScope,
        what: &str,
        start: impl FnOnce() -> F,
    ) -> (bool, bool)
    where
        F: Future<Output = Result<Box<D>, String>> + Send + 'static,
    {
        let started = self.read.is_pending() && self.read.start(start());
        let Some(result) = self.read.take_ready(scope).await else {
            return (started, false);
        };
        self.detail = None;
        self.error = None;
        match result {
            Ok(detail) => self.detail = Some(detail),
            Err(error) => self.error = Some(clean(&error.into_message(what))),
        }
        (true, true)
    }
}

pub(crate) fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

pub(crate) fn timestamp(text: &str) -> String {
    clean(&crate::common::format_iso_timestamp(text))
}

/// `active_impaired` -> `Active impaired`, `round_robin` -> `Round robin`.
pub(crate) fn words(raw: &str) -> String {
    let text = raw.replace(['_', '-', '.'], " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub(crate) fn lb_type_name(raw: &str) -> String {
    match raw {
        "application" => "Application".into(),
        "network" => "Network".into(),
        "gateway" => "Gateway".into(),
        other => other.into(),
    }
}

pub(crate) fn scheme_name(raw: &str) -> String {
    match raw {
        "internet-facing" => "Internet-facing".into(),
        "internal" => "Internal".into(),
        other => other.into(),
    }
}

pub(crate) fn ip_type_name(raw: &str) -> String {
    match raw {
        "ipv4" => "IPv4".into(),
        "ipv6" => "IPv6".into(),
        "dualstack" => "Dualstack".into(),
        "dualstack-without-public-ipv4" => "Dualstack without public IPv4".into(),
        other => other.into(),
    }
}

pub(crate) fn target_type_name(raw: &str) -> String {
    match raw {
        "instance" => "Instance".into(),
        "ip" => "IP".into(),
        "lambda" => "Lambda".into(),
        "alb" => "Application Load Balancer".into(),
        other => other.into(),
    }
}

pub(crate) fn health_name(raw: &str) -> String {
    match raw {
        "unhealthy.draining" => "Unhealthy (draining)".into(),
        other => words(other),
    }
}

/// Green for a good state, yellow for a passing one, red for a bad one.
pub(crate) fn style(text: &str) -> Style {
    let color = match text {
        "Active" | "Healthy" | "On" | "Enabled" => Some(Color::Green),
        "Provisioning" | "Active impaired" | "Initial" | "Draining" | "Unused" | "Off"
        | "Disabled" => Some(Color::Yellow),
        "Failed" | "Unhealthy" | "Unhealthy (draining)" | "Unavailable" => Some(Color::Red),
        _ => None,
    };
    color.map_or_else(Style::default, |c| Style::default().fg(c))
}

#[derive(Clone, Copy)]
enum Kind {
    Flag,
    Seconds,
    Words,
}

/// Console labels of the attributes, in console order.
const ATTRIBUTES: &[(&str, &str, Kind)] = &[
    (
        "deletion_protection.enabled",
        "Deletion protection",
        Kind::Flag,
    ),
    (
        "load_balancing.cross_zone.enabled",
        "Cross-zone load balancing",
        Kind::Flag,
    ),
    (
        "idle_timeout.timeout_seconds",
        "Connection idle timeout",
        Kind::Seconds,
    ),
    (
        "client_keep_alive.seconds",
        "HTTP client keepalive duration",
        Kind::Seconds,
    ),
    ("routing.http2.enabled", "HTTP/2", Kind::Flag),
    (
        "routing.http.drop_invalid_header_fields.enabled",
        "Drop invalid header fields",
        Kind::Flag,
    ),
    (
        "routing.http.desync_mitigation_mode",
        "Desync mitigation mode",
        Kind::Words,
    ),
    (
        "routing.http.preserve_host_header.enabled",
        "Preserve host header",
        Kind::Flag,
    ),
    (
        "routing.http.xff_header_processing.mode",
        "X-Forwarded-For header",
        Kind::Words,
    ),
    (
        "routing.http.xff_client_port.enabled",
        "Client port preservation",
        Kind::Flag,
    ),
    (
        "routing.http.x_amzn_tls_version_and_cipher_suite.enabled",
        "TLS version and cipher headers",
        Kind::Flag,
    ),
    ("waf.fail_open.enabled", "WAF fail open", Kind::Flag),
    ("access_logs.s3.enabled", "Access logs", Kind::Flag),
    ("connection_logs.s3.enabled", "Connection logs", Kind::Flag),
    (
        "health_check_logs.s3.enabled",
        "Health check logs",
        Kind::Flag,
    ),
    (
        "zonal_shift.config.enabled",
        "Zonal shift integration",
        Kind::Flag,
    ),
    (
        "dns_record.client_routing_policy",
        "Client routing policy (DNS record)",
        Kind::Words,
    ),
    (
        "deregistration_delay.timeout_seconds",
        "Deregistration delay",
        Kind::Seconds,
    ),
    (
        "slow_start.duration_seconds",
        "Slow start duration",
        Kind::Seconds,
    ),
    (
        "load_balancing.algorithm.type",
        "Load balancing algorithm",
        Kind::Words,
    ),
    ("stickiness.enabled", "Stickiness", Kind::Flag),
    ("stickiness.type", "Stickiness type", Kind::Words),
    (
        "preserve_client_ip.enabled",
        "Preserve client IP addresses",
        Kind::Flag,
    ),
    ("proxy_protocol_v2.enabled", "Proxy protocol v2", Kind::Flag),
    (
        "deregistration_delay.connection_termination.enabled",
        "Connection termination on deregistration",
        Kind::Flag,
    ),
    (
        "lambda.multi_value_headers.enabled",
        "Multi value headers",
        Kind::Flag,
    ),
];

/// Attributes as the console labels them: known keys in console order with
/// `On`/`Off` and units, then any other key as the API names it.
pub(crate) fn attribute_fields(attributes: &[(String, String)]) -> Vec<(String, String)> {
    let shown = |kind: Kind, raw: &str| match (kind, raw) {
        (_, "") => "-".to_string(),
        (Kind::Flag, "true") => "On".into(),
        (Kind::Flag, "false") => "Off".into(),
        (Kind::Seconds, n) => format!("{} seconds", clean(n)),
        (Kind::Words, w) => words(&clean(w)),
        (Kind::Flag, other) => clean(other),
    };
    let mut fields: Vec<(String, String)> = ATTRIBUTES
        .iter()
        .filter_map(|(key, label, kind)| {
            let (_, raw) = attributes.iter().find(|(k, _)| k == key)?;
            Some((label.to_string(), shown(*kind, raw)))
        })
        .collect();
    let mut rest: Vec<_> = attributes
        .iter()
        .filter(|(k, _)| !ATTRIBUTES.iter().any(|(key, ..)| key == k))
        .map(|(k, v)| (clean(k), if v.is_empty() { "-".into() } else { clean(v) }))
        .collect();
    rest.sort();
    fields.extend(rest);
    fields
}

/// True on an entered load balancer or target group.
pub(crate) fn detail_active(app: &App) -> bool {
    (load_balancers::active(app) && app.load_balancers_state.view.is_some())
        || (target_groups::active(app) && app.target_groups_state.view.is_some())
}

/// Keys every service keeps for itself.
pub(crate) fn global(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::ConfirmQuit
            | Action::CloseService
            | Action::NextTab
            | Action::PrevTab
            | Action::OpenSpaceMenu
            | Action::ShowHelp
            | Action::CopyToClipboard
            | Action::CopyCliCommand
            | Action::OpenCliOutput
            | Action::OpenRelatedResources
            | Action::OpenResourceFinder
            | Action::OpenInBrowser
            | Action::ToggleBookmark
            | Action::OpenBookmarkPicker
            | Action::OpenInConsole
    )
}

/// Split the borrows the shared table input needs out of `app`.
macro_rules! keys {
    ($app:expr, $field:ident, $selected:expr) => {{
        let App {
            $field,
            mode,
            page_input,
            error_message,
            error_scroll,
            clipboard,
            ..
        } = $app;
        (
            $field,
            crate::spec_table::Keys {
                mode,
                page_input,
                error_message,
                error_scroll,
                clipboard,
                selected_arn: $selected,
            },
        )
    }};
}
pub(crate) use keys;

/// Ctrl+R in the error modal: leave it; the caller then refreshes.
pub(crate) fn retry_from_error(app: &mut App, action: &Action) -> bool {
    if *action == Action::RetryLoad {
        app.error_message = None;
        app.error_scroll = 0;
        app.mode = crate::keymap::Mode::Normal;
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
