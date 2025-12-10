use crate::app::App;
use crate::common::{render_pagination_text, CyclicEnum, InputFocus, SortDirection};
use crate::ecr::image::{self, Image as EcrImage};
use crate::ecr::repo::{self, Repository as EcrRepository};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::render_tabs;
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use ratatui::{prelude::*, widgets::*};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct State {
    pub repositories: TableState<EcrRepository>,
    pub tab: Tab,
    pub current_repository: Option<String>,
    pub current_repository_uri: Option<String>,
    pub images: TableState<EcrImage>,
    pub input_focus: InputFocus,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            repositories: TableState::new(),
            tab: Tab::Private,
            current_repository: None,
            current_repository_uri: None,
            images: TableState::new(),
            input_focus: InputFocus::Filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Private,
    Public,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[Self::Private, Self::Public];
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::Private => "Private",
            Tab::Public => "Public",
        }
    }
}

pub fn filtered_ecr_repositories(app: &App) -> Vec<&EcrRepository> {
    if app.ecr_state.repositories.filter.is_empty() {
        app.ecr_state.repositories.items.iter().collect()
    } else {
        app.ecr_state
            .repositories
            .items
            .iter()
            .filter(|r| {
                r.name
                    .to_lowercase()
                    .contains(&app.ecr_state.repositories.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn filtered_ecr_images(app: &App) -> Vec<&EcrImage> {
    if app.ecr_state.images.filter.is_empty() {
        app.ecr_state.images.items.iter().collect()
    } else {
        app.ecr_state
            .images
            .items
            .iter()
            .filter(|img| {
                img.tag
                    .to_lowercase()
                    .contains(&app.ecr_state.images.filter.to_lowercase())
                    || img
                        .digest
                        .to_lowercase()
                        .contains(&app.ecr_state.images.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn render_repositories(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.ecr_state.current_repository.is_some() {
        render_images(frame, app, area);
    } else {
        render_repository_list(frame, app, area);
    }
}

pub fn render_repository_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tabs
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ])
        .split(area);

    // Tabs
    let tabs: Vec<(&str, Tab)> = Tab::ALL.iter().map(|tab| (tab.name(), *tab)).collect();
    render_tabs(frame, chunks[0], &tabs, &app.ecr_state.tab);

    // Calculate pagination
    let filtered_count: usize = app
        .ecr_state
        .repositories
        .items
        .iter()
        .filter(|r| {
            app.ecr_state.repositories.filter.is_empty()
                || r.name
                    .to_lowercase()
                    .contains(&app.ecr_state.repositories.filter.to_lowercase())
        })
        .count();

    let page_size = app.ecr_state.repositories.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.ecr_state.repositories.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    // Filter
    render_simple_filter(
        frame,
        chunks[1],
        SimpleFilterConfig {
            filter_text: &app.ecr_state.repositories.filter,
            placeholder: "Search by repository substring",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.ecr_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.ecr_state.input_focus == InputFocus::Pagination,
        },
    );

    // Table
    let filtered: Vec<_> = app
        .ecr_state
        .repositories
        .items
        .iter()
        .filter(|r| {
            app.ecr_state.repositories.filter.is_empty()
                || r.name
                    .to_lowercase()
                    .contains(&app.ecr_state.repositories.filter.to_lowercase())
        })
        .collect();

    // Apply pagination
    let page_size = app.ecr_state.repositories.page_size.value();
    let current_page = app.ecr_state.repositories.selected / page_size;
    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let tab_label = match app.ecr_state.tab {
        Tab::Private => "Private",
        Tab::Public => "Public",
    };
    let title = format!(" {} repositories ({}) ", tab_label, filtered.len());

    // Define columns
    let columns: Vec<Box<dyn Column<EcrRepository>>> = app
        .ecr_repo_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            repo::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<EcrRepository>>)
        })
        .collect();

    let expanded_index = app.ecr_state.repositories.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: paginated,
        selected_index: app.ecr_state.repositories.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Repository name",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[2],
        get_expanded_content: Some(Box::new(|repo: &EcrRepository| {
            expanded_from_columns(&columns, repo)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

pub fn render_images(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ])
        .split(area);

    // Calculate pagination
    let filtered_count: usize = app
        .ecr_state
        .images
        .items
        .iter()
        .filter(|img| {
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
        .count();

    let page_size = 50;
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.ecr_state.images.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.ecr_state.images.filter,
            placeholder: "Search artifacts",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: true,
            is_pagination_focused: false,
        },
    );

    // Table
    let filtered: Vec<_> = app
        .ecr_state
        .images
        .items
        .iter()
        .filter(|img| {
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
        .collect();

    // Apply pagination
    let page_size = app.ecr_state.repositories.page_size.value();
    let current_page = app.ecr_state.images.selected / page_size;
    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let title = format!(" Images ({}) ", filtered.len());

    // Define columns
    let columns: Vec<Box<dyn Column<EcrImage>>> = app
        .ecr_image_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            image::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<EcrImage>>)
        })
        .collect();

    let config = TableConfig {
        items: paginated,
        selected_index: app.ecr_state.images.selected - app.ecr_state.images.scroll_offset,
        expanded_index: app
            .ecr_state
            .images
            .expanded_item
            .map(|idx| idx - app.ecr_state.images.scroll_offset),
        columns: &columns,
        sort_column: "Pushed at",
        sort_direction: SortDirection::Desc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|img: &EcrImage| {
            expanded_from_columns(&columns, img)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}
