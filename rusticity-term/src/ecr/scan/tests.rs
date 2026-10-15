use super::*;
use crate::{ecr::image::Image, keymap::handle_key};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::ecr::scan::Finding;

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::EcrRepositories;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.ecr_state.current_repository = Some("nested/repo".into());
    app.ecr_state.current_repository_uri =
        Some("123456789012.dkr.ecr.us-east-1.amazonaws.com/nested/repo".into());
    app.ecr_state.images.items = (0..3)
        .map(|i| Image {
            tag: format!("tag{i}"),
            digest: format!("sha256:{i}"),
            artifact_type: "application/vnd.docker.container.image.v1+json".into(),
            pushed_at: "2026-09-09T23:12:21Z".into(),
            size_bytes: 473640150,
            uri: String::new(),
            last_pull_time: "2026-09-09T23:21:00Z".into(),
        })
        .collect();
    app
}
fn press(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    let action = handle_key(KeyEvent::new(key, modifiers), app.mode).unwrap();
    app.handle_action(action);
}
fn enter(app: &mut App) {
    press(app, KeyCode::Enter, KeyModifiers::NONE);
}
fn tab(app: &mut App) {
    press(app, KeyCode::Tab, KeyModifiers::NONE);
}
fn screen(app: &App, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 45)).unwrap();
    terminal.draw(|f| ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}
fn details() -> ImageDetails {
    ImageDetails {
        tags: vec!["v2-cargo".into(), "latest".into()],
        digest: "sha256:0".into(),
        artifact_type: "application/vnd.docker.container.image.v1+json".into(),
        manifest_type: "application/vnd.docker.distribution.manifest.v2+json".into(),
        pushed_at: "2026-09-09T23:12:21Z".into(),
        last_pull_time: "2026-09-09T23:21:00Z".into(),
        size_bytes: Some(473640150),
        image_status: "ACTIVE".into(),
    }
}
async fn finish(app: &mut App, result: Result<Output, String>) {
    app.ecr_state.scan.pending = None;
    let tab = app.ecr_state.scan.view.as_ref().unwrap().tab;
    app.ecr_state.scan.running = Some(Running {
        tab,
        task: tokio::spawn(async { result }),
    });
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}

#[test]
fn image_enter_opens_general_not_scan_and_back_preserves_image_list() {
    let mut app = app();
    app.ecr_state.images.filter = "tag2".into();
    app.ecr_state.images.expanded_item = Some(0);
    assert!(screen(&app, 180).contains("image details"));
    enter(&mut app);
    let page = app.ecr_state.scan.view.as_ref().unwrap();
    assert_eq!(page.digest, "sha256:2");
    assert_eq!(page.tab, DetailTab::General);
    assert_eq!(app.ecr_state.scan.pending, Some(DetailTab::General));
    assert!(page.scan.is_none());
    assert!(!screen(&app, 180).contains("Scan availability:"));
    let crumbs = crate::ecr::actions::breadcrumb(&app);
    assert!(crumbs.contains(&"sha256:2".into()));
    assert!(crumbs.contains(&"General information".into()));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.ecr_state.scan.view.is_none());
    assert_eq!(app.ecr_state.images.filter, "tag2");
    assert_eq!(app.ecr_state.images.expanded_item, Some(0));
    assert_eq!(
        app.ecr_state.current_repository.as_deref(),
        Some("nested/repo")
    );
}

#[tokio::test]
async fn details_header_and_general_pane_use_real_metadata_and_all_tags() {
    let mut app = app();
    enter(&mut app);
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    for width in [80, 160, 220] {
        let text = screen(&app, width);
        for field in [
            "Details",
            "Image tags",
            "v2-cargo, latest",
            "Digest",
            "sha256:0",
            "General information",
            "Artifact type",
            "Repository",
            "Size (MB)",
            "473.64",
            "Image status",
            "Active",
        ] {
            assert!(text.contains(field), "missing {field} at {width}");
        }
    }
    assert_eq!(
        view::copy_uri(&app),
        "123456789012.dkr.ecr.us-east-1.amazonaws.com/nested/repo:v2-cargo"
    );
    assert!(screen(&app, 180).contains("2026-09-09 23:12:21 (UTC)"));
    assert!(!screen(&app, 180).contains("snapshot"));
    assert!(!app.ecr_state.scan.busy());
    assert!(app.ecr_state.scan.view.as_ref().unwrap().scan.is_none());
}

#[tokio::test]
async fn untagged_images_use_digest_uri_and_unknown_fields_are_not_fabricated() {
    let mut app = app();
    enter(&mut app);
    let mut data = details();
    data.tags.clear();
    data.size_bytes = None;
    data.image_status = "-".into();
    finish(&mut app, Ok(Output::Metadata(data))).await;
    assert!(view::copy_uri(&app).ends_with("repo@sha256:0"));
    let text = screen(&app, 180);
    assert!(text.contains("Image tags: Untagged"));
    assert!(text.contains("Size (MB): -"));
    assert!(text.contains("Image status: -"));
}

#[tokio::test]
async fn ribbon_cycles_five_tabs_and_only_loads_uncached_selected_tab() {
    let mut app = app();
    enter(&mut app);
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    for expected in [
        DetailTab::Scanning,
        DetailTab::Referrers,
        DetailTab::Replication,
        DetailTab::Signing,
    ] {
        tab(&mut app);
        assert_eq!(app.ecr_state.scan.view.as_ref().unwrap().tab, expected);
        assert_eq!(app.ecr_state.scan.pending, Some(expected));
        assert!(crate::ecr::actions::breadcrumb(&app).contains(&expected.name().into()));
    }
    tab(&mut app);
    assert_eq!(
        app.ecr_state.scan.view.as_ref().unwrap().tab,
        DetailTab::General
    );
    assert!(!app.ecr_state.scan.busy());
    press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(
        app.ecr_state.scan.view.as_ref().unwrap().tab,
        DetailTab::Signing
    );
    assert_eq!(app.ecr_state.tab, Tab::Private);
}

#[tokio::test]
async fn scan_summary_zeros_require_a_completed_scan_and_findings_expand() {
    let mut app = app();
    enter(&mut app);
    tab(&mut app);
    finish(
        &mut app,
        Ok(Output::Scan(ScanReport {
            status: "COMPLETE".into(),
            description: "The scan was completed successfully.".into(),
            completed_at: "2026-09-09T23:13:35Z".into(),
            source_updated_at: "2026-09-09T23:13:35Z".into(),
            has_findings_payload: true,
            ..ScanReport::default()
        })),
    )
    .await;
    let text = screen(&app, 180);
    for field in [
        "Status: Complete",
        "Critical: 0",
        "High: 0",
        "Medium: 0",
        "Low: 0",
        "Info: 0",
        "No findings reported by this scan",
    ] {
        assert!(text.contains(field), "missing {field}");
    }
    let mut report = ScanReport {
        status: "IN_PROGRESS".into(),
        has_findings_payload: true,
        ..ScanReport::default()
    };
    finish(&mut app, Ok(Output::Scan(report))).await;
    let text = screen(&app, 180);
    assert!(text.contains("Critical: -"));
    assert!(!text.contains("No findings reported"));
    report = ScanReport {
        status: "COMPLETE".into(),
        has_findings_payload: true,
        ..ScanReport::default()
    };
    report.findings.push(Finding {
        severity: "HIGH".into(),
        name: "CVE-test".into(),
        description: "Description".into(),
        details: vec![
            ("Package".into(), "openssl".into()),
            ("Fixed in".into(), "1.2".into()),
        ],
    });
    finish(&mut app, Ok(Output::Scan(report))).await;
    enter(&mut app);
    let text = screen(&app, 180);
    assert!(text.contains("CVE-test"));
    assert!(text.contains("Package: openssl"));
    assert!(text.contains("Fixed in: 1.2"));
    assert!(text.contains("Critical: -")); // No summary supplied; do not invent zero.
}

#[tokio::test]
async fn tables_distinguish_no_data_from_denied_or_partial_data() {
    let mut app = app();
    enter(&mut app);
    for (tab, empty) in [
        (DetailTab::Referrers, "No referrers"),
        (DetailTab::Replication, "No replication statuses"),
        (DetailTab::Signing, "No signing statuses"),
    ] {
        app.ecr_state.scan.activate(tab, false);
        finish(
            &mut app,
            Ok(Output::Report(DetailReport {
                entries: vec![],
                warning: None,
            })),
        )
        .await;
        assert!(screen(&app, 220).contains(empty));
        finish(
            &mut app,
            Ok(Output::Report(DetailReport {
                entries: vec![],
                warning: Some("Unavailable: AccessDeniedException".into()),
            })),
        )
        .await;
        let text = screen(&app, 220);
        assert!(!text.contains(empty));
        assert!(text.contains("AccessDeniedException"));
    }
    app.ecr_state.scan.activate(DetailTab::Scanning, false);
    finish(&mut app, Err("ScanNotFoundException: not scanned".into())).await;
    let text = screen(&app, 180);
    assert!(text.contains("Status: No scan"));
    assert!(!text.contains("Critical: 0"));
}

#[tokio::test]
async fn replication_source_dropdown_queries_only_after_confirm_and_status_filter_is_local() {
    let mut app = app();
    enter(&mut app);
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    app.ecr_state.scan.activate(DetailTab::Replication, false);
    finish(
        &mut app,
        Ok(Output::Report(DetailReport {
            entries: vec![
                vec![
                    ("Target account".into(), "111111111111".into()),
                    ("Status".into(), "COMPLETE".into()),
                ],
                vec![
                    ("Target account".into(), "222222222222".into()),
                    ("Status".into(), "FAILED".into()),
                ],
            ],
            warning: None,
        })),
    )
    .await;
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    for c in "failed".chars() {
        press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
    }
    enter(&mut app);
    assert_eq!(app.mode, Mode::Normal);
    let page = app.ecr_state.scan.view.as_ref().unwrap();
    assert_eq!(page.data().filtered(page.tab).len(), 1);
    assert_eq!(
        page.data().filtered(page.tab)[0].field("Target account"),
        "222222222222"
    );
    assert!(!app.ecr_state.scan.busy());
    press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
    assert!(screen(&app, 180).contains("digest (immutable)"));
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert!(!app.ecr_state.scan.busy());
    enter(&mut app);
    let page = app.ecr_state.scan.view.as_ref().unwrap();
    assert_eq!(page.source_tag.as_deref(), Some("v2-cargo"));
    assert!(page.data().table.items.is_empty());
    assert_eq!(app.ecr_state.scan.pending, Some(DetailTab::Replication));
    assert!(screen(&app, 180).contains("tag: v2-cargo (mutable)"));
}

#[tokio::test]
async fn tasks_cancel_on_tab_change_escape_and_context_changes() {
    for change in 0..9 {
        let mut app = app();
        enter(&mut app);
        app.ecr_state.scan.pending = None;
        let task = tokio::spawn(std::future::pending::<Result<Output, String>>());
        let abort = task.abort_handle();
        app.ecr_state.scan.running = Some(Running {
            tab: DetailTab::General,
            task,
        });
        match change {
            0 => app.profile = "other".into(),
            1 => app.config.region = "eu-west-1".into(),
            2 => app.config.account_id = "other".into(),
            3 => app.config.role_arn = "other".into(),
            4 => app.current_service = Service::S3Buckets,
            5 => app.ecr_state.current_repository = Some("other".into()),
            6 => app.current_tab += 1,
            7 => tab(&mut app),
            _ => press(&mut app, KeyCode::Esc, KeyModifiers::NONE),
        }
        if change < 7 {
            assert!(poll(&mut app).await);
        }
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
        if change != 7 {
            assert!(app.ecr_state.scan.view.is_none());
        }
    }
}

#[test]
fn invalid_selection_and_public_images_never_queue_private_reads() {
    let mut app = app();
    app.ecr_state.images.selected = usize::MAX;
    enter(&mut app);
    assert!(app.ecr_state.scan.view.is_none());
    app.ecr_state.images.selected = 0;
    app.ecr_state.tab = Tab::Public;
    enter(&mut app);
    assert!(!app.ecr_state.scan.busy());
    tab(&mut app);
    assert!(!app.ecr_state.scan.busy());
    assert!(screen(&app, 180).contains("private ECR"));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    app.ecr_state.tab = Tab::Private;
    app.ecr_state.images.items[0].digest.clear();
    enter(&mut app);
    assert!(!app.ecr_state.scan.busy());
    assert!(screen(&app, 180).contains("digest unavailable"));
}

#[tokio::test]
async fn refresh_is_per_tab_and_render_strips_control_characters() {
    let mut app = app();
    enter(&mut app);
    let mut metadata = details();
    metadata.tags[0] = "evil\x1b\ntest".into();
    finish(&mut app, Ok(Output::Metadata(metadata))).await;
    assert!(!view::copy_uri(&app).contains('\x1b'));
    app.ecr_state.scan.activate(DetailTab::Signing, false);
    finish(
        &mut app,
        Ok(Output::Report(DetailReport {
            entries: vec![],
            warning: None,
        })),
    )
    .await;
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(app.ecr_state.scan.pending, Some(DetailTab::Signing));
    assert!(app.ecr_state.scan.view.as_ref().unwrap().metadata.is_some());
    assert!(!app.ecr_state.repositories.loading);
}

#[tokio::test]
async fn details_reuses_shared_three_column_labeled_pane() {
    let mut app = app();
    enter(&mut app);
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    let mut terminal = Terminal::new(TestBackend::new(300, 30)).unwrap();
    terminal
        .draw(|f| {
            render(f, &app, f.area());
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let line: String = (0..300).map(|x| buffer[(x, 1)].symbol()).collect();
    assert!(line.contains("Image tags: v2-cargo, latest"));
    assert!(line.contains("URI: "));
    assert!(line.contains("Digest: sha256:0"));
    assert_eq!(buffer[(0, 2)].symbol(), "╰");
}

#[tokio::test]
async fn source_dropdown_is_inline_and_filter_tab_focus_uses_shared_control() {
    let mut app = app();
    enter(&mut app);
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    app.ecr_state.scan.activate(DetailTab::Replication, false);
    finish(
        &mut app,
        Ok(Output::Report(DetailReport {
            entries: vec![],
            warning: None,
        })),
    )
    .await;
    let mut terminal = Terminal::new(TestBackend::new(180, 35)).unwrap();
    terminal
        .draw(|f| {
            render(f, &app, f.area());
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let lines: Vec<String> = (0..35)
        .map(|y| (0..180).map(|x| buffer[(x, y)].symbol()).collect())
        .collect();
    let filter = lines
        .iter()
        .find(|line| line.contains("Filter by status"))
        .unwrap();
    assert!(filter.contains("Select status source: digest"));
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    tab(&mut app);
    assert_eq!(app.mode, Mode::FilterInput);
    assert!(app.ecr_state.scan.view.as_ref().unwrap().selecting_source());
    let text = screen(&app, 180);
    assert!(text.contains("digest (immutable)"));
    assert!(text.contains("tag: latest"));
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    enter(&mut app);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .source_tag
            .as_deref(),
        Some("v2-cargo")
    );
    assert_eq!(app.ecr_state.scan.pending, Some(DetailTab::Replication));
}

fn loaded_table(tab: DetailTab) -> App {
    let mut app = app();
    enter(&mut app);
    app.ecr_state.scan.activate(tab, false);
    app.ecr_state.scan.pending = None;
    let view = app.ecr_state.scan.view.as_mut().unwrap();
    view.data_mut().loaded = true;
    view.data_mut().table.items = (0..125)
        .map(|i| {
            Row(vec![
                ("Status".into(), "COMPLETE".into()),
                ("Target account".into(), i.to_string()),
            ])
        })
        .collect();
    app
}

#[test]
fn replication_focus_cycle_includes_pagination_and_wraps_both_directions() {
    use crate::common::InputFocus;
    let mut app = loaded_table(DetailTab::Replication);
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    for expected in [input::SOURCE, InputFocus::Pagination, InputFocus::Filter] {
        tab(&mut app);
        let view = app.ecr_state.scan.view.as_ref().unwrap();
        assert_eq!(view.data().focus, expected);
        assert_eq!(view.selecting_source(), expected == input::SOURCE);
        assert_eq!(app.mode, Mode::FilterInput);
        assert!(!app.ecr_state.scan.busy());
    }
    for expected in [InputFocus::Pagination, input::SOURCE, InputFocus::Filter] {
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        let view = app.ecr_state.scan.view.as_ref().unwrap();
        assert_eq!(view.data().focus, expected);
        assert_eq!(view.selecting_source(), expected == input::SOURCE);
    }
}

#[test]
fn shared_pagination_handles_arrows_page_numbers_and_focus_highlight() {
    use crate::common::InputFocus;
    let mut app = loaded_table(DetailTab::Replication);
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    tab(&mut app);
    tab(&mut app);
    assert_eq!(
        app.ecr_state.scan.view.as_ref().unwrap().data().focus,
        InputFocus::Pagination
    );
    let mut terminal = Terminal::new(TestBackend::new(180, 35)).unwrap();
    terminal
        .draw(|f| {
            render(f, &app, f.area());
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let y = (0..35)
        .find(|y| {
            let line: String = (0..180).map(|x| buffer[(x, *y)].symbol()).collect();
            line.contains("Select status source")
        })
        .unwrap();
    let x = (0..180).find(|x| buffer[(*x, y)].symbol() == "[").unwrap();
    assert_eq!(buffer[(x, y)].fg, ratatui::style::Color::Yellow);
    app.ecr_state
        .scan
        .view
        .as_mut()
        .unwrap()
        .data_mut()
        .table
        .expanded_item = Some(0);
    for selected in [50, 100, 100] {
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        let view = app.ecr_state.scan.view.as_ref().unwrap();
        assert_eq!(view.data().table.selected, selected);
        assert_eq!(view.data().table.scroll_offset, selected);
        assert!(view.data().table.expanded_item.is_none());
    }
    press(&mut app, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .table
            .selected,
        50
    );
    press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
    assert_eq!(app.page_input, "3");
    assert!(app
        .ecr_state
        .scan
        .view
        .as_ref()
        .unwrap()
        .data()
        .table
        .filter
        .is_empty());
    enter(&mut app);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .table
            .selected,
        100
    );
    assert!(app.page_input.is_empty());
    assert!(!app.ecr_state.scan.busy());
}

#[test]
fn all_detail_tables_use_common_filter_pagination_focus_and_reset_on_insert() {
    use crate::common::InputFocus;
    for tab_kind in [
        DetailTab::Scanning,
        DetailTab::Referrers,
        DetailTab::Signing,
    ] {
        let mut app = loaded_table(tab_kind);
        press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        tab(&mut app);
        assert_eq!(
            app.ecr_state.scan.view.as_ref().unwrap().data().focus,
            InputFocus::Pagination
        );
        tab(&mut app);
        assert_eq!(
            app.ecr_state.scan.view.as_ref().unwrap().data().focus,
            InputFocus::Filter
        );
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(
            app.ecr_state.scan.view.as_ref().unwrap().data().focus,
            InputFocus::Pagination
        );
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(
            app.ecr_state.scan.view.as_ref().unwrap().data().focus,
            InputFocus::Filter
        );
    }
}

#[test]
fn source_shortcut_tab_uses_same_cycle_without_committing_highlighted_tag() {
    use crate::common::InputFocus;
    let mut app = loaded_table(DetailTab::Replication);
    app.ecr_state.scan.view.as_mut().unwrap().metadata = Some(details());
    press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    tab(&mut app);
    let view = app.ecr_state.scan.view.as_ref().unwrap();
    assert_eq!(app.mode, Mode::FilterInput);
    assert_eq!(view.data().focus, InputFocus::Pagination);
    assert!(view.source_tag.is_none());
    assert!(!view.selecting_source());
    assert!(!app.ecr_state.scan.busy());
}

#[tokio::test]
async fn general_pane_omits_informational_footer_but_keeps_loading_and_error_notices() {
    let mut app = app();
    enter(&mut app);
    assert!(screen(&app, 180).contains("Loading image metadata"));
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    app.ecr_state.scan.view.as_mut().unwrap().scan = Some(ScanReport {
        status: "COMPLETE".into(),
        has_findings_payload: true,
        ..ScanReport::default()
    });
    let text = screen(&app, 180);
    assert!(!text.contains("Scan availability:"));
    assert!(!text.contains("refreshed by ECR"));
    assert!(!text.contains("Loading image metadata"));
    assert!(text.contains("Last recorded pull time:"));
    finish(&mut app, Err("AccessDeniedException: denied".into())).await;
    assert!(screen(&app, 180).contains("Metadata unavailable: AccessDeniedException"));
}

mod preferences;
