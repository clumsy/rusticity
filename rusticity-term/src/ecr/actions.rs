use crate::app::{App, Service};
use crate::common::{CyclicEnum, InputFocus};
use crate::preferences::bindings;
use crate::ui::ecr::{filtered_ecr_images, filtered_ecr_repositories, FILTER_CONTROLS};

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        let filtered = filtered_ecr_images(app);
        if !filtered.is_empty() {
            app.ecr_state.images.next_item(filtered.len());
        }
    } else {
        let filtered = filtered_ecr_repositories(app);
        if !filtered.is_empty() {
            app.ecr_state.repositories.selected =
                (app.ecr_state.repositories.selected + 1).min(filtered.len() - 1);
            app.ecr_state.repositories.snap_to_page();
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.prev_item();
    } else {
        app.ecr_state.repositories.prev_item();
    }
}

pub fn page_down_filter_input(app: &mut App) {
    if app.ecr_state.input_focus == InputFocus::Filter {
        let filtered = filtered_ecr_repositories(app);
        app.ecr_state.repositories.page_down(filtered.len());
    } else {
        let page_size = app.ecr_state.repositories.page_size.value();
        let filtered_count = filtered_ecr_repositories(app).len();
        app.ecr_state.input_focus.handle_page_down(
            &mut app.ecr_state.repositories.selected,
            &mut app.ecr_state.repositories.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        let filtered = filtered_ecr_images(app);
        app.ecr_state.images.page_down(filtered.len());
    } else {
        let filtered = filtered_ecr_repositories(app);
        app.ecr_state.repositories.page_down(filtered.len());
    }
}

pub fn page_up_filter_input(app: &mut App) {
    if app.ecr_state.input_focus == InputFocus::Filter {
        app.ecr_state.repositories.page_up();
    } else {
        let page_size = app.ecr_state.repositories.page_size.value();
        app.ecr_state.input_focus.handle_page_up(
            &mut app.ecr_state.repositories.selected,
            &mut app.ecr_state.repositories.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.page_up();
    } else {
        app.ecr_state.repositories.page_up();
    }
}

pub fn next_pane(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.toggle_expand();
    } else {
        app.ecr_state.repositories.toggle_expand();
    }
}

pub fn prev_pane(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.collapse();
    } else {
        app.ecr_state.repositories.collapse();
    }
}

pub fn collapse_row(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.collapse();
    } else {
        app.ecr_state.repositories.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    if app.ecr_state.current_repository.is_some() {
        let filtered_count = filtered_ecr_images(app).len();
        app.ecr_state.images.goto_page(page, filtered_count);
    } else {
        let filtered_count = filtered_ecr_repositories(app).len();
        app.ecr_state.repositories.goto_page(page, filtered_count);
    }
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.reset();
    } else {
        app.ecr_state.repositories.reset();
    }
}

pub fn start_filter(app: &mut App) {
    if app.ecr_state.current_repository.is_none() {
        app.ecr_state.input_focus = InputFocus::Filter;
    }
}

pub fn next_filter_focus(app: &mut App) {
    app.ecr_state.input_focus = app.ecr_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.ecr_state.input_focus = app.ecr_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.ecr_state.current_repository.is_none()
        && app.ecr_state.input_focus == InputFocus::Pagination
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.ecr_state.current_repository.is_none() {
        let filtered_repos = filtered_ecr_repositories(app);
        if let Some(repo) = app.ecr_state.repositories.get_selected(&filtered_repos) {
            let repo_name = repo.name.clone();
            let repo_uri = repo.uri.clone();
            app.ecr_state.current_repository = Some(repo_name);
            app.ecr_state.current_repository_uri = Some(repo_uri);
            app.ecr_state.images.reset();
            app.ecr_state.repositories.loading = true;
        }
    }
}

pub fn go_back(app: &mut App) {
    if app.ecr_state.images.has_expanded_item() {
        app.ecr_state.images.collapse();
    } else {
        app.ecr_state.current_repository = None;
        app.ecr_state.current_repository_uri = None;
        app.ecr_state.images.items.clear();
        app.ecr_state.images.reset();
    }
}

pub fn next_detail_tab(app: &mut App) {
    app.ecr_state.tab = app.ecr_state.tab.next();
    app.ecr_state.repositories.reset();
    app.ecr_state.repositories.loading = true;
}

pub fn prev_detail_tab(app: &mut App) {
    app.ecr_state.tab = app.ecr_state.tab.prev();
    app.ecr_state.repositories.reset();
    app.ecr_state.repositories.loading = true;
}

pub fn yank(app: &App) {
    if app.ecr_state.current_repository.is_some() {
        let filtered = filtered_ecr_images(app);
        if let Some(image) = app.ecr_state.images.get_selected(&filtered) {
            app.clipboard.request_text(&image.uri);
        }
    } else {
        let filtered = filtered_ecr_repositories(app);
        if let Some(repo) = app.ecr_state.repositories.get_selected(&filtered) {
            app.clipboard.request_text(&repo.uri);
        }
    }
}

pub fn scroll_up(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_state.images.page_up();
    } else {
        app.ecr_state.repositories.page_up();
    }
}

pub fn scroll_down(app: &mut App) {
    if app.ecr_state.current_repository.is_some() {
        let filtered = filtered_ecr_images(app);
        app.ecr_state.images.page_down(filtered.len());
    } else {
        let filtered = filtered_ecr_repositories(app);
        app.ecr_state.repositories.page_down(filtered.len());
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    bindings::activate_for(app, Service::EcrRepositories);
}

pub fn next_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::EcrRepositories, true);
}

pub fn prev_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::EcrRepositories, false);
}

pub fn column_selector_max(app: &App) -> usize {
    bindings::model_for(app, Service::EcrRepositories).max_index()
}

pub fn column_count(app: &App) -> usize {
    bindings::model_for(app, Service::EcrRepositories).column_count()
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["ECR".to_string()];
    if let Some(repo) = &app.ecr_state.current_repository {
        parts.push(repo.clone());
        if let Some(view) = &app.ecr_state.scan.view {
            parts.push(view.digest.clone());
            parts.push(view.tab.name().into());
        }
    } else {
        parts.push("Repositories".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::ecr;
    if let Some(repo_name) = &app.ecr_state.current_repository {
        if app.ecr_state.tab == crate::ui::ecr::Tab::Public {
            return ecr::console_url_public_repository(&app.config.region, repo_name);
        }
        if let Some(view) = &app.ecr_state.scan.view {
            if !view.digest.is_empty() {
                return ecr::console_url_private_image(
                    &app.config.region,
                    &app.config.account_id,
                    repo_name,
                    &view.digest,
                );
            }
        }
        ecr::console_url_private_repository(&app.config.region, &app.config.account_id, repo_name)
    } else {
        ecr::console_url_repositories(&app.config.region)
    }
}

#[cfg(test)]
mod console_tests {
    use super::*;
    use crate::{
        ecr::image::Image,
        keymap::{Action, Mode},
    };

    fn image_app() -> App {
        let mut app = App::new_without_client("test".into(), Some("eu-west-1".into()));
        app.config.account_id = "123456789012".into();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.current_repository = Some("team/image".into());
        app.ecr_state.images.items.push(Image {
            tag: "latest".into(),
            digest: "sha256:abc123".into(),
            artifact_type: String::new(),
            pushed_at: String::new(),
            size_bytes: 0,
            uri: String::new(),
            last_pull_time: String::new(),
        });
        app
    }

    #[test]
    fn every_image_detail_tab_links_to_the_open_image_not_repository_or_mutable_tag() {
        let mut app = image_app();
        assert!(!app.get_console_url().contains("/_/image/"));
        app.handle_action(Action::Select);
        // The open page's digest remains authoritative if the inventory changes.
        app.ecr_state.images.items[0].digest = "sha256:changed".into();
        let expected = crate::ecr::console_url_private_image(
            "eu-west-1",
            "123456789012",
            "team/image",
            "sha256:abc123",
        );
        for _ in 0..5 {
            assert_eq!(app.get_console_url(), expected);
            app.handle_action(Action::NextDetailTab);
        }
        app.handle_action(Action::GoBack);
        assert!(!app.get_console_url().contains("/_/image/"));
    }

    #[test]
    fn missing_digest_and_public_context_do_not_create_private_image_links() {
        let mut app = image_app();
        app.ecr_state.images.items[0].digest.clear();
        app.handle_action(Action::Select);
        assert!(!app.get_console_url().contains("/_/image/"));
        app.handle_action(Action::GoBack);
        app.ecr_state.tab = crate::ui::ecr::Tab::Public;
        app.handle_action(Action::Select);
        assert!(app.get_console_url().contains("/repositories/public/"));
        assert!(!app.get_console_url().contains("/repositories/private/"));
    }
}
