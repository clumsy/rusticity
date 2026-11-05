//! App-level tests for the ACM inventory: registration, tab-owned loading,
//! fresh-only refresh and failure, cancellation, columns, filter, console,
//! CLI, preferences and background tabs. No test makes an AWS request.
use super::*;
use crate::{
    app::{App, Service, Tab},
    async_read::ReadStatus,
    keymap::{handle_key, Action, Mode},
    tab_runtime::{self, status_for, TabId},
    ui::acm::filtered_certificates,
};
use columns::Column;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use rusticity_core::Certificate;

fn cert(id: &str, domain: &str) -> Certificate {
    Certificate {
        arn: format!("arn:aws:acm:us-east-1:123456789012:certificate/{id}"),
        id: id.into(),
        domain_name: domain.into(),
        additional_names: vec![domain.into(), format!("www.{domain}")],
        cert_type: "AMAZON_ISSUED".into(),
        status: "ISSUED".into(),
        in_use: Some(true),
        renewal_eligibility: "ELIGIBLE".into(),
        key_algorithm: "RSA_2048".into(),
        not_after: "2027-01-02T03:04:05Z".into(),
        ..Default::default()
    }
}

fn rows() -> Vec<Certificate> {
    vec![
        cert("cert-b", "beta.example"),
        cert("cert-a", "alpha.example"),
        Certificate {
            status: "PENDING_VALIDATION".into(),
            in_use: None,
            ..cert("cert-p", "pending.example")
        },
    ]
}

/// An ACM tab bound in the tab runtime, with its first read queued.
fn app() -> (App, TabId) {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    let id = TabId::new();
    app.tabs.push(Tab {
        id,
        service: Service::AcmCertificates,
        title: "ACM › Certificates".into(),
        breadcrumb: "ACM › Certificates".into(),
    });
    app.current_tab = 0;
    app.current_service = Service::AcmCertificates;
    app.service_selected = true;
    app.mode = Mode::Normal;
    tab_runtime::sync_active(&mut app);
    (app, id)
}

async fn finish(app: &mut App, result: Result<Vec<Certificate>, String>) {
    assert!(app.acm_state.inventory.start(async move { result }));
    tokio::task::yield_now().await;
    assert!(inventory::poll(app).await);
}

async fn loaded() -> (App, TabId) {
    let (mut app, id) = app();
    finish(&mut app, Ok(rows())).await;
    (app, id)
}

fn press(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    if let Some(action) = handle_key(KeyEvent::new(key, mods), app.mode) {
        app.handle_action(action);
    }
}

fn screen(app: &App) -> String {
    let mut t = Terminal::new(TestBackend::new(200, 30)).unwrap();
    t.draw(|f| crate::ui::render(f, app)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn service_is_registered_once_and_sorted_first_in_the_picker() {
    let app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    assert_eq!(Service::AcmCertificates.name(), "ACM › Certificates");
    assert_eq!(app.service_picker.services[0], "ACM › Certificates");
    assert_eq!(
        app.service_picker
            .services
            .iter()
            .filter(|s| **s == "ACM › Certificates")
            .count(),
        1
    );
}

#[test]
fn opening_from_the_picker_creates_a_tab_that_queues_a_read() {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.mode = Mode::ServicePicker;
    app.service_picker.filter = "ACM".into();
    app.handle_action(Action::Select);
    assert_eq!(app.current_service, Service::AcmCertificates);
    tab_runtime::sync_active(&mut app);
    let id = app.tabs[app.current_tab].id;
    assert!(matches!(status_for(&app, id), ReadStatus::Loading { .. }));
    assert_eq!(app.breadcrumbs(), "ACM > Certificates");
}

#[tokio::test]
async fn loading_keeps_filter_and_headers_and_shows_spinner_in_the_tab() {
    let (app, _) = app();
    let text = screen(&app);
    assert!(text.contains("Search by domain"));
    assert!(text.contains("Domain name") && text.contains("Loading…"));
    assert!(text.contains("Loading certificates"), "status bar message");
    assert!(!text.contains("Certificates (0)"));
}

#[tokio::test]
async fn success_shows_fresh_rows_sorted_by_domain_with_console_labels() {
    let (app, id) = loaded().await;
    assert_eq!(status_for(&app, id), ReadStatus::Ready);
    let domains: Vec<_> = filtered_certificates(&app)
        .iter()
        .map(|c| c.domain_name.as_str())
        .collect();
    assert_eq!(
        domains,
        ["alpha.example", "beta.example", "pending.example"]
    );
    let text = screen(&app);
    assert!(text.contains("Certificates (3)") && text.contains("ACM › Certificates ✓"));
    assert!(text.contains("Amazon issued") && text.contains("Issued"));
    assert!(text.contains("Pending validation") && text.contains("Eligible"));
    assert!(text.contains("RSA 2048"));
}

#[test]
fn columns_render_console_labels_and_never_invent_values() {
    let mut labels = std::collections::HashMap::new();
    init(&mut labels);
    for column in Column::ALL {
        assert!(column.id().starts_with("column.acm.certificate."));
        assert_eq!(Column::from_id(column.id()), Some(column));
        assert_eq!(labels[column.id()], column.default_name());
    }
    let names: Vec<_> = Column::default_visible_ids()
        .into_iter()
        .map(|id| Column::from_id(id).unwrap().default_name())
        .collect();
    assert_eq!(
        names,
        [
            "Certificate ID",
            "Domain name",
            "Type",
            "Status",
            "In use",
            "Renewal eligibility",
            "Key algorithm"
        ]
    );
    let empty = Certificate::default();
    for column in Column::ALL {
        let text = column.render(&empty).0;
        match column {
            // The console shows No rather than a blank for these flags.
            Column::InUse | Column::Exported => assert_eq!(text, "No"),
            _ => assert_eq!(text, "-", "{column:?}"),
        }
    }
    let c = cert("id-1", "example.com");
    assert_eq!(Column::InUse.render(&c).0, "Yes");
    assert_eq!(Column::AdditionalNames.render(&c).0, "www.example.com");
    let truncated = Certificate {
        more_names: true,
        ..c.clone()
    };
    assert_eq!(
        Column::AdditionalNames.render(&truncated).0,
        "www.example.com, …"
    );
    assert_eq!(Column::NotAfter.render(&c).0, "2027-01-02 03:04:05 (UTC)");
    assert_eq!(
        Column::Status.render(&c).1.fg,
        Some(ratatui::style::Color::Green)
    );
    let hostile = Certificate {
        domain_name: "bad\x1b[31m.example".into(),
        ..Default::default()
    };
    assert!(!Column::DomainName.render(&hostile).0.contains('\x1b'));
}

#[tokio::test]
async fn filter_matches_the_displayed_text() {
    let (mut app, _) = loaded().await;
    app.acm_state.certificates.filter = "pending validation".into();
    let rows = filtered_certificates(&app);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "cert-p");
    app.acm_state.certificates.filter = "cert-a".into();
    assert_eq!(filtered_certificates(&app)[0].domain_name, "alpha.example");
    app.acm_state.certificates.filter = "www.beta".into();
    assert_eq!(filtered_certificates(&app)[0].id, "cert-b");
}

#[tokio::test]
async fn empty_success_says_so_only_after_the_read() {
    let (mut app, id) = app();
    assert!(!screen(&app).contains("No certificates in this Region."));
    finish(&mut app, Ok(vec![])).await;
    assert_eq!(status_for(&app, id), ReadStatus::Ready);
    assert!(screen(&app).contains("No certificates in this Region."));
}

#[tokio::test]
async fn refresh_clears_rows_before_the_next_draw_and_failure_never_restores_them() {
    let (mut app, id) = loaded().await;
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(app.acm_state.certificates.items.is_empty());
    assert!(matches!(status_for(&app, id), ReadStatus::Loading { .. }));
    let text = screen(&app);
    assert!(!text.contains("alpha.example") && text.contains("Loading…"));

    finish(
        &mut app,
        Err("AccessDeniedException: acm:ListCertificates".into()),
    )
    .await;
    assert_eq!(status_for(&app, id), ReadStatus::Failed);
    let text = screen(&app);
    assert!(text.contains("AccessDeniedException") && !text.contains("alpha.example"));
    assert!(text.contains("ACM › Certificates !"));
}

#[tokio::test]
async fn esc_cancels_the_read_without_showing_older_rows() {
    let (mut app, id) = loaded().await;
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(app
        .acm_state
        .inventory
        .start(async { std::future::pending::<Result<Vec<Certificate>, String>>().await }));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.acm_state.inventory.is_busy());
    assert_eq!(status_for(&app, id), ReadStatus::Failed);
    let text = screen(&app);
    assert!(text.contains("Cancelled") && !text.contains("alpha.example"));
}

#[tokio::test]
async fn aws_context_change_drops_rows() {
    let (mut app, _) = loaded().await;
    app.config.region = "eu-west-1".into();
    assert!(inventory::poll(&mut app).await);
    assert!(app.acm_state.certificates.items.is_empty());
}

#[tokio::test]
async fn console_and_cli_follow_the_table_not_the_highlighted_row() {
    let (mut app, _) = loaded().await;
    let list =
        "https://us-east-1.console.aws.amazon.com/acm/home?region=us-east-1#/certificates/list";
    assert_eq!(app.get_console_url(), list);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    app.acm_state.certificates.filter = "beta".into();
    assert_eq!(app.get_console_url(), list);
    let command = crate::cli_command::invocation(&app).unwrap().display;
    assert!(
        command.starts_with("aws acm list-certificates "),
        "{command}"
    );
    assert!(!command.contains("cert-b") && !command.contains("get-certificate"));
    assert!(command.contains("--region='us-east-1'") && command.contains("--profile='beta'"));
}

#[tokio::test]
async fn copy_takes_the_selected_filtered_certificate_arn() {
    let (mut app, _) = loaded().await;
    app.acm_state.certificates.filter = "beta".into();
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    let mut copied = None;
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            copied = Some(text);
            Ok(())
        }));
    assert_eq!(
        copied.as_deref(),
        Some("arn:aws:acm:us-east-1:123456789012:certificate/cert-b")
    );
}

#[tokio::test]
async fn preferences_expose_every_column_and_hide_one() {
    use crate::preferences::bindings::model_for;
    let (mut app, _) = loaded().await;
    assert_eq!(model_for(&app, Service::AcmCertificates).column_count(), 16);
    app.acm_state
        .visible_column_ids
        .retain(|id| *id != Column::KeyAlgorithm.id());
    assert!(!screen(&app).contains("Key algorithm"));
}

#[tokio::test]
async fn certificates_are_not_bookmarkable_without_a_detail_view() {
    let (app, _) = loaded().await;
    assert!(crate::finder::related::selected(&app).is_none());
    assert!(!crate::bookmarks::available(&app));
    assert!(!screen(&app).contains("bookmark"));
}

#[tokio::test]
async fn background_tab_finishes_without_stealing_focus() {
    let (mut app, first) = app();
    let (tx, rx) = tokio::sync::oneshot::channel();
    assert!(app
        .acm_state
        .inventory
        .start(async move { rx.await.unwrap() }));
    let second = TabId::new();
    app.tabs.push(Tab {
        id: second,
        service: Service::AcmCertificates,
        title: "Second".into(),
        breadcrumb: "Second".into(),
    });
    app.current_tab = 1;
    tab_runtime::sync_active(&mut app);
    assert!(matches!(
        status_for(&app, first),
        ReadStatus::Loading { .. }
    ));
    assert!(matches!(
        status_for(&app, second),
        ReadStatus::Loading { .. }
    ));

    tx.send(Ok(rows())).unwrap();
    tokio::task::yield_now().await;
    assert!(tab_runtime::poll(&mut app).await);
    assert_eq!(app.current_tab, 1);
    assert_eq!(status_for(&app, first), ReadStatus::Ready);
    assert!(
        app.acm_state.certificates.items.is_empty(),
        "second tab owns its own rows"
    );
    app.handle_action(Action::PrevTab);
    assert_eq!(app.acm_state.certificates.items.len(), 3);
}
