use crate::app::App;
use crate::common::{
    filter_by_field, render_pagination_text, CyclicEnum, InputFocus, SortDirection,
};
use crate::ecr::image::{self, Image as EcrImage};
use crate::ecr::repo::{self, Repository as EcrRepository};
use crate::keymap::Mode;
use crate::table::{TableState, TableWindow};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use crate::ui::{format_title, render_tabs};
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
    filter_by_field(
        &app.ecr_state.repositories.items,
        &app.ecr_state.repositories.filter,
        |r| &r.name,
    )
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

    let tabs: Vec<(&str, Tab)> = Tab::ALL.iter().map(|tab| (tab.name(), *tab)).collect();
    render_tabs(frame, chunks[0], &tabs, &app.ecr_state.tab);

    let view = app
        .ecr_state
        .repositories
        .prepare_view(filtered_ecr_repositories(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);

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

    let tab_label = match app.ecr_state.tab {
        Tab::Private => "Private",
        Tab::Public => "Public",
    };
    let title = format_title(&format!(
        "{} repositories ({})",
        tab_label, view.total_items
    ));

    let columns: Vec<Box<dyn Column<EcrRepository>>> = app
        .ecr_repo_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            repo::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<EcrRepository>>)
        })
        .collect();

    let config = TableConfig {
        items: view.items,
        selected_index: view.selected_index,
        expanded_index: view.expanded_index,
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
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let view = app
        .ecr_state
        .images
        .prepare_view(filtered_ecr_images(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
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
    let columns: Vec<Box<dyn Column<EcrImage>>> = app
        .ecr_image_visible_column_ids
        .iter()
        .filter_map(|id| {
            image::Column::from_id(id).map(|col| Box::new(col) as Box<dyn Column<EcrImage>>)
        })
        .collect();
    render_table(
        frame,
        TableConfig {
            items: view.items,
            selected_index: view.selected_index,
            expanded_index: view.expanded_index,
            columns: &columns,
            sort_column: "Pushed at",
            sort_direction: SortDirection::Desc,
            title: format_title(&format!("Images ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|image| expanded_from_columns(&columns, image))),
            is_active: app.mode != Mode::FilterInput,
        },
    );
}

#[cfg(test)]
mod prepared_image_tests {
    use super::*;
    use crate::common::PageSize;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn images_use_their_own_page_size_and_safe_relative_selection() {
        let mut app = App::new_without_client("test".into(), None);
        app.ecr_state.repositories.page_size = PageSize::OneHundred;
        app.ecr_state.images.page_size = PageSize::Ten;
        app.ecr_state.images.items = (0..25)
            .map(|i| EcrImage {
                tag: format!("tag{i:02}"),
                digest: format!("digest{i:02}"),
                artifact_type: "Image".into(),
                pushed_at: String::new(),
                size_bytes: 0,
                uri: String::new(),
                last_pull_time: String::new(),
            })
            .collect();
        app.ecr_state.images.selected = 12;
        app.ecr_state.images.expanded_item = Some(11);
        app.ecr_state.images.scroll_offset = 200; // Legacy scroll offsets must not underflow.
        let mut terminal = Terminal::new(TestBackend::new(240, 40)).unwrap();
        terminal.draw(|f| render_images(f, &app, f.area())).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("1 [2] 3"));
        assert!(text.contains("► tag12"));
        assert!(text.contains("▼ tag11"));
        assert!(!text.contains("tag09"));
        assert!(!text.contains("tag20"));
        app.ecr_state.images.filter = "no match".into();
        terminal.draw(|f| render_images(f, &app, f.area())).unwrap();
    }
}
