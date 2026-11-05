//! Search, match navigation and sideways scroll for the shared JSON/text
//! viewer, through the real key map and App action pipeline.
use super::*;
use crate::{
    app::{App, Service, Tab, ViewMode},
    keymap::handle_key,
    tab_runtime::TabId,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::TestBackend,
    style::{Color, Style},
    text::Span,
    Terminal,
};

const KEY_ARN: &str = "arn:aws:kms:us-east-1:123456789012:key/1234abcd-12ab-34cd-56ef-1234567890ab";

/// A long policy: the word `Sid` appears on lines 3, 103 and 203.
fn long_policy() -> String {
    let mut lines = vec!["{".to_string(), "  \"Statement\": [".to_string()];
    for block in 0..3 {
        lines.push(format!("    {{ \"Sid\": \"Block{block}\","));
        for i in 0..99 {
            lines.push(format!("      \"Line{block}_{i}\": \"value-{block}-{i}\","));
        }
    }
    lines.push(format!(
        "    \"Wide\": \"{}needle-at-the-far-right\"",
        "x".repeat(300)
    ));
    lines.push("  ]".into());
    lines.push("}".into());
    lines.join("\n")
}

fn kms_app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    app.tabs.push(Tab {
        id: TabId::new(),
        service: Service::KmsKeys,
        title: "KMS › Managed Keys".into(),
        breadcrumb: "KMS › Managed Keys".into(),
    });
    app.current_service = Service::KmsKeys;
    app.service_selected = true;
    app.mode = Mode::Normal;
    crate::tab_runtime::sync_active(&mut app);
    app.kms_state.inventory.cancel();
    app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;
    app.kms_state.keys.items = vec![crate::kms::key::Key {
        key_id: "1234abcd-12ab-34cd-56ef-1234567890ab".into(),
        key_arn: KEY_ARN.into(),
        alias: "prod".into(),
        description: String::new(),
        key_state: "Enabled".into(),
        key_usage: String::new(),
        key_spec: String::new(),
        key_manager: "Customer".into(),
        creation_date: String::new(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: String::new(),
        enabled: true,
        multi_region: false,
    }];
    app.kms_state.inventory = Default::default();
    press(&mut app, KeyCode::Enter);
    app.kms_state
        .detail
        .finish_with_policy_for_test(&long_policy());
    while app.kms_state.detail.view().unwrap().tab != crate::kms::detail::Tab::Policy {
        press(&mut app, KeyCode::Tab);
    }
    // Render once so the viewer records its viewport size.
    screen(&app);
    app
}

fn press(app: &mut App, code: KeyCode) {
    let modifiers = if matches!(code, KeyCode::Char(c) if c.is_uppercase()) {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    if let Some(action) = handle_key(KeyEvent::new(code, modifiers), app.mode) {
        app.handle_action(action);
    }
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

fn screen(app: &App) -> (String, ratatui::buffer::Buffer) {
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (
        buffer.content().iter().map(|c| c.symbol()).collect(),
        buffer,
    )
}

fn policy_scroll(app: &App) -> usize {
    app.kms_state.detail.view().unwrap().policy_scroll
}

#[test]
fn matching_is_literal_smart_case_and_counts_every_occurrence() {
    let text = "Allow allow ALLOW\nnone\nüber Über";
    let lower = find_matches(text, "allow");
    assert_eq!(lower.len(), 3, "lowercase query ignores case");
    assert_eq!(
        lower[1],
        Match {
            line: 0,
            start: 6,
            len: 5
        }
    );
    assert_eq!(
        find_matches(text, "Allow").len(),
        1,
        "uppercase means exact"
    );
    assert!(find_matches(text, "").is_empty());
    assert!(find_matches(text, "a.l").is_empty(), "no regex");
    let umlaut = find_matches(text, "über");
    assert_eq!(umlaut.len(), 2);
    assert_eq!(umlaut[1].start, 5, "offsets are characters, not bytes");
    assert_eq!(find_matches("aaaa", "aa").len(), 2, "non-overlapping");
}

#[test]
fn highlighters_reproduce_each_line_so_match_offsets_line_up() {
    let json = long_policy();
    let yaml = "Resources:\n  Bucket:\n    Type: AWS::S3::Bucket # comment\n  - item\n";
    for (text, highlight) in [
        (json.as_str(), Highlight::Json),
        (yaml, Highlight::Template),
        ("plain text\twith tab", Highlight::Plain),
    ] {
        let line_fn = highlight.line_fn(text);
        for line in text.lines() {
            let joined: String = line_fn(line).iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(joined, line);
        }
    }
}

#[test]
fn marks_patch_syntax_style_and_keep_text_intact() {
    let spans = vec![
        Span::styled("\"key\"", Style::default().fg(Color::Blue)),
        Span::raw(": \"value\""),
    ];
    let bg = Style::default().bg(Color::Yellow);
    let marked = crate::ui::mark_spans_for_test(spans.clone(), &[(1, 4, bg), (8, 13, bg)]);
    let joined: String = marked.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(joined, "\"key\": \"value\"");
    let key = marked.iter().find(|s| s.content == "key").unwrap();
    assert_eq!(key.style.fg, Some(Color::Blue), "syntax color kept");
    assert_eq!(key.style.bg, Some(Color::Yellow));
    let value = marked.iter().find(|s| s.content == "value").unwrap();
    assert_eq!(value.style.bg, Some(Color::Yellow));
    let skipped = crate::ui::skip_chars_for_test(marked, 3);
    let joined: String = skipped.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(joined, "y\": \"value\"");
}

#[test]
fn typing_a_query_finds_live_matches_and_enter_keeps_it() {
    let mut app = kms_app();
    let (text, _) = screen(&app);
    assert!(
        text.contains("/ search · ←→ scroll"),
        "hint in the viewer border"
    );
    press(&mut app, KeyCode::Char('/'));
    assert_eq!(app.mode, Mode::ViewerSearch);
    type_text(&mut app, "sid");
    let (text, _) = screen(&app);
    assert!(text.contains("/sid▏ 1 of 3"), "live count while typing");
    assert!(text.contains("keep search"), "search-mode status hints");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.mode, Mode::Normal);
    let (text, _) = screen(&app);
    assert!(text.contains("/sid  1 of 3  n/N next/prev"));
}

#[test]
fn n_and_shift_n_jump_between_matches_wrap_and_reveal_them() {
    let mut app = kms_app();
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "sid");
    press(&mut app, KeyCode::Enter);
    assert!(policy_scroll(&app) <= 2, "first match is already visible");
    press(&mut app, KeyCode::Char('n'));
    let scroll = policy_scroll(&app);
    assert!(
        scroll <= 102 && 102 < scroll + 38,
        "line 103 revealed, scroll {scroll}"
    );
    let (text, buffer) = screen(&app);
    assert!(text.contains("2 of 3"));
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.bg == Color::Yellow && cell.symbol() == "S"),
        "current match is highlighted"
    );
    press(&mut app, KeyCode::Char('n'));
    press(&mut app, KeyCode::Char('n'));
    assert!(
        screen(&app).0.contains("1 of 3"),
        "wraps past the last match"
    );
    press(&mut app, KeyCode::Char('N'));
    assert!(screen(&app).0.contains("3 of 3"), "N wraps backwards");
}

#[test]
fn esc_while_typing_restores_the_previous_query() {
    let mut app = kms_app();
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "sid");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "nothing-matches");
    assert!(screen(&app).0.contains("No matches"));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
    assert!(screen(&app).0.contains("/sid"));
}

#[test]
fn esc_clears_an_active_query_before_going_back() {
    let mut app = kms_app();
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "sid");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Esc);
    assert!(crate::kms::detail::active(&app), "first Esc only clears");
    assert!(!screen(&app).0.contains("/sid"));
    press(&mut app, KeyCode::Esc);
    assert!(!crate::kms::detail::active(&app), "second Esc goes back");
}

#[test]
fn arrows_scroll_sideways_and_a_far_right_match_is_revealed() {
    let mut app = kms_app();
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.text_viewer.hscroll, 2 * HSCROLL_STEP);
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Left);
    assert_eq!(app.text_viewer.hscroll, 0, "never below zero");

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "needle");
    press(&mut app, KeyCode::Enter);
    assert!(app.text_viewer.hscroll > 250, "jumped right to the match");
    assert!(screen(&app).0.contains("needle-at-the-far-right"));
}

#[test]
fn keys_do_nothing_when_no_viewer_is_focused() {
    let mut app = kms_app();
    press(&mut app, KeyCode::Tab); // Policy → Tags (a table)
    press(&mut app, KeyCode::Char('/'));
    assert_eq!(app.mode, Mode::Normal);
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.text_viewer.hscroll, 0);
}

#[test]
fn search_state_resets_for_another_viewer() {
    let mut app = kms_app();
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "sid");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Right);
    // Same text, different viewer: an SQS access policy in a new tab.
    app.tabs.push(Tab {
        id: TabId::new(),
        service: Service::SqsQueues,
        title: "SQS".into(),
        breadcrumb: "SQS".into(),
    });
    app.current_tab = 1;
    app.current_service = Service::SqsQueues;
    crate::tab_runtime::sync_active(&mut app);
    app.sqs_state.current_queue = Some("https://sqs.example/q".into());
    app.sqs_state.detail_tab = crate::ui::sqs::QueueDetailTab::QueuePolicies;
    app.sqs_state.policy_document = long_policy();
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.text_viewer.hscroll, 0);
    assert!(!screen(&app).0.contains("/sid"));
}

/// Every registered viewer: the search finds a far-down match and moves that
/// view's own scroll field to it.
#[test]
fn every_registered_viewer_searches_and_scrolls_its_own_pane() {
    type Setup = fn(&mut App, String);
    type Scroll = fn(&App) -> usize;
    let cases: Vec<(&str, Service, Setup, Scroll)> = vec![
        (
            "SQS access policy",
            Service::SqsQueues,
            |app, text| {
                app.sqs_state.current_queue = Some("https://sqs.example/q".into());
                app.sqs_state.detail_tab = crate::ui::sqs::QueueDetailTab::QueuePolicies;
                app.sqs_state.policy_document = text;
            },
            |app| app.sqs_state.policy_scroll,
        ),
        (
            "EFS policy",
            Service::EfsFileSystems,
            |app, text| {
                app.efs_state.file_systems.items = vec![crate::efs::fs::FileSystem {
                    file_system_id: "fs-1".into(),
                    file_system_arn: String::new(),
                    name: String::new(),
                    creation_token: String::new(),
                    encrypted: String::new(),
                    kms_key_id: String::new(),
                    total_size: String::new(),
                    size_in_standard: String::new(),
                    size_in_ia: String::new(),
                    size_in_archive: String::new(),
                    provisioned_throughput: String::new(),
                    throughput_mode: String::new(),
                    life_cycle_state: String::new(),
                    number_of_mount_targets: String::new(),
                    owner_id: String::new(),
                    creation_time: String::new(),
                    performance_mode: String::new(),
                    availability_zone: String::new(),
                    replication_overwrite_protection: String::new(),
                    total_size_bytes: 0,
                    size_in_standard_bytes: 0,
                    size_in_ia_bytes: 0,
                    size_in_archive_bytes: 0,
                    dns_name: String::new(),
                    tags: Vec::new(),
                }];
                app.efs_state.current_file_system = Some("fs-1".into());
                app.efs_state.detail_tab = crate::ui::efs::DetailTab::FileSystemPolicy;
                app.efs_state.policy_document = text;
            },
            |app| app.efs_state.policy_scroll,
        ),
        (
            "IAM trust policy",
            Service::IamRoles,
            |app, text| {
                app.iam_state.current_role = Some("role".into());
                app.iam_state.role_tab = crate::ui::iam::RoleTab::TrustRelationships;
                app.iam_state.trust_policy_document = text;
            },
            |app| app.iam_state.trust_policy_scroll,
        ),
        (
            "IAM policy document",
            Service::IamRoles,
            |app, text| {
                app.view_mode = ViewMode::PolicyView;
                app.iam_state.policy_document = text;
            },
            |app| app.iam_state.policy_scroll,
        ),
        (
            "CloudFormation template",
            Service::CloudFormationStacks,
            |app, text| {
                app.cfn_state.table.items = vec![crate::cfn::Stack {
                    name: "stack".into(),
                    stack_id: "id".into(),
                    status: "CREATE_COMPLETE".into(),
                    created_time: String::new(),
                    updated_time: String::new(),
                    deleted_time: String::new(),
                    drift_status: String::new(),
                    last_drift_check_time: String::new(),
                    status_reason: String::new(),
                    description: String::new(),
                    detailed_status: String::new(),
                    root_stack: String::new(),
                    parent_stack: String::new(),
                    termination_protection: false,
                    iam_role: String::new(),
                    tags: Vec::new(),
                    stack_policy: String::new(),
                    rollback_monitoring_time: String::new(),
                    rollback_alarms: Vec::new(),
                    notification_arns: Vec::new(),
                }];
                app.cfn_state.current_stack = Some("stack".into());
                app.cfn_state.detail_tab = crate::ui::cfn::DetailTab::Template;
                app.cfn_state.template_body = text;
            },
            |app| app.cfn_state.template_scroll,
        ),
        (
            "CloudTrail event record",
            Service::CloudTrailEvents,
            |app, text| {
                app.cloudtrail_state.current_event = Some(crate::cloudtrail::CloudTrailEvent {
                    event_time: String::new(),
                    event_name: String::new(),
                    username: String::new(),
                    event_source: String::new(),
                    resource_type: String::new(),
                    resource_name: String::new(),
                    read_only: String::new(),
                    aws_region: String::new(),
                    event_id: "evt-1".into(),
                    access_key_id: String::new(),
                    source_ip_address: String::new(),
                    error_code: String::new(),
                    request_id: String::new(),
                    event_type: String::new(),
                    cloud_trail_event_json: text,
                });
                app.cloudtrail_state.detail_focus = crate::app::CloudTrailDetailFocus::EventRecord;
            },
            |app| app.cloudtrail_state.event_json_scroll,
        ),
    ];
    for (name, service, setup, scroll) in cases {
        let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
        app.tabs.push(Tab {
            id: TabId::new(),
            service,
            title: name.into(),
            breadcrumb: name.into(),
        });
        app.current_service = service;
        app.service_selected = true;
        app.mode = Mode::Normal;
        crate::tab_runtime::sync_active(&mut app);
        setup(&mut app, long_policy());
        screen(&app);
        press(&mut app, KeyCode::Char('/'));
        assert_eq!(app.mode, Mode::ViewerSearch, "{name}: / starts a search");
        type_text(&mut app, "Block2");
        press(&mut app, KeyCode::Enter);
        let top = scroll(&app);
        assert!(
            top <= 202 && 202 < top + 30,
            "{name}: scrolled to line 203, got {top}"
        );
        assert!(screen(&app).0.contains("/Block2  1 of 1"), "{name}: footer");
    }
}

#[test]
fn efs_policy_without_its_file_system_row_is_not_a_hidden_viewer() {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::EfsFileSystems;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.efs_state.current_file_system = Some("fs-missing".into());
    app.efs_state.detail_tab = crate::ui::efs::DetailTab::FileSystemPolicy;
    app.efs_state.policy_document = long_policy();
    press(&mut app, KeyCode::Char('/'));
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn cloudtrail_resources_pane_focus_keeps_its_own_arrows() {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::CloudTrailEvents;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.cloudtrail_state.current_event = None;
    press(&mut app, KeyCode::Char('/'));
    assert_eq!(app.mode, Mode::Normal, "no event record, no viewer");
}
