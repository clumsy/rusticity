use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
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
        let filtered_count = app
            .ecr_state
            .images
            .filtered(|img| {
                app.ecr_state.images.filter.is_empty()
                    || img
                        .tag
                        .to_lowercase()
                        .contains(&app.ecr_state.images.filter.to_lowercase())
                    || img
                        .digest
                        .to_lowercase()
                        .contains(&app.ecr_state.images.filter.to_lowercase())
            })
            .len();
        app.ecr_state.images.goto_page(page, filtered_count);
    } else {
        let filtered_count = app
            .ecr_state
            .repositories
            .filtered(|r| {
                app.ecr_state.repositories.filter.is_empty()
                    || r.name
                        .to_lowercase()
                        .contains(&app.ecr_state.repositories.filter.to_lowercase())
            })
            .len();
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
    use crate::app::copy_to_clipboard;
    if app.ecr_state.current_repository.is_some() {
        let filtered = filtered_ecr_images(app);
        if let Some(image) = app.ecr_state.images.get_selected(&filtered) {
            copy_to_clipboard(&image.uri);
        }
    } else {
        let filtered = filtered_ecr_repositories(app);
        if let Some(repo) = app.ecr_state.repositories.get_selected(&filtered) {
            copy_to_clipboard(&repo.uri);
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
    let idx = app.column_selector_index;
    if app.ecr_state.current_repository.is_some() {
        // Images view — idx is 0-based
        if let Some(col) = app.ecr_image_column_ids.get(idx) {
            if let Some(pos) = app
                .ecr_image_visible_column_ids
                .iter()
                .position(|c| c == col)
            {
                if app.ecr_image_visible_column_ids.len() > 1 {
                    app.ecr_image_visible_column_ids.remove(pos);
                }
            } else {
                app.ecr_image_visible_column_ids.push(*col);
            }
        }
    } else {
        // Repositories view — idx is 1-based (0 = header)
        if idx > 0 && idx <= app.ecr_repo_column_ids.len() {
            if let Some(col) = app.ecr_repo_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .ecr_repo_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.ecr_repo_visible_column_ids.len() > 1 {
                        app.ecr_repo_visible_column_ids.remove(pos);
                    }
                } else {
                    app.ecr_repo_visible_column_ids.push(*col);
                }
            }
        } else if idx == app.ecr_repo_column_ids.len() + 3 {
            app.ecr_state.repositories.page_size = PageSize::Ten;
        } else if idx == app.ecr_repo_column_ids.len() + 4 {
            app.ecr_state.repositories.page_size = PageSize::TwentyFive;
        } else if idx == app.ecr_repo_column_ids.len() + 5 {
            app.ecr_state.repositories.page_size = PageSize::Fifty;
        } else if idx == app.ecr_repo_column_ids.len() + 6 {
            app.ecr_state.repositories.page_size = PageSize::OneHundred;
        }
    }
}

pub fn next_preferences(app: &mut App) {
    // Images view only — repos view handled by generic logic
    let page_size_idx = app.ecr_image_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let page_size_idx = app.ecr_image_column_ids.len() + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_image_column_ids.len() + 6
    } else {
        app.ecr_repo_column_ids.len() + 6
    }
}

pub fn column_count(app: &App) -> usize {
    if app.ecr_state.current_repository.is_some() {
        app.ecr_image_column_ids.len()
    } else {
        app.ecr_repo_column_ids.len()
    }
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["ECR".to_string()];
    if let Some(repo) = &app.ecr_state.current_repository {
        parts.push(repo.clone());
    } else {
        parts.push("Repositories".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::ecr;
    if let Some(repo_name) = &app.ecr_state.current_repository {
        ecr::console_url_private_repository(&app.config.region, &app.config.account_id, repo_name)
    } else {
        ecr::console_url_repositories(&app.config.region)
    }
}
