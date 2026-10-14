//! Shared key/value tag filtering, rendering, input routing and preferences.
use crate::app::{App, Service};
use crate::common::{render_pagination_text, InputFocus, PageSize, SortDirection};
use crate::inventory::{self, InventoryInput};
use crate::keymap::{Action, Mode};
use crate::preferences::{PreferenceColumn, Preferences, PAGE_SIZE_OPTIONS};
use crate::table::{TableState, TableWindow};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use crate::ui::{format_title, titled_block};
use ratatui::{prelude::*, widgets::Paragraph};

pub const COLUMN_NAMES: [&str; 2] = ["Key", "Value"];
const SMALL_PAGE_SIZES: &[(PageSize, &str)] = &[
    (PageSize::Ten, "10"),
    (PageSize::TwentyFive, "25"),
    (PageSize::Fifty, "50"),
];

pub trait Tag {
    fn key(&self) -> &str;
    fn value(&self) -> &str;
}
impl Tag for (String, String) {
    fn key(&self) -> &str {
        &self.0
    }
    fn value(&self) -> &str {
        &self.1
    }
}
macro_rules! impl_tag {
    ($t:ty) => {
        impl Tag for $t {
            fn key(&self) -> &str {
                &self.key
            }
            fn value(&self) -> &str {
                &self.value
            }
        }
    };
}
impl_tag!(crate::iam::RoleTag);
impl_tag!(crate::iam::UserTag);
impl_tag!(crate::ec2::tag::InstanceTag);

#[derive(Clone, Copy)]
pub enum TagSort {
    Source,
    Key,
    Value,
}

pub fn filtered<'a, T: Tag>(items: &'a [T], filter: &str, sort: TagSort) -> Vec<&'a T> {
    let query = filter.to_lowercase();
    let mut rows: Vec<_> = items
        .iter()
        .filter(|t| {
            query.is_empty()
                || t.key().to_lowercase().contains(&query)
                || t.value().to_lowercase().contains(&query)
        })
        .collect();
    match sort {
        TagSort::Source => {}
        TagSort::Key => rows.sort_by(|a, b| a.key().cmp(b.key())),
        TagSort::Value => rows.sort_by(|a, b| a.value().cmp(b.value())),
    }
    rows
}

pub enum TagColumn {
    Key,
    Value,
}
impl<T: Tag> Column<T> for TagColumn {
    fn name(&self) -> &str {
        match self {
            Self::Key => COLUMN_NAMES[0],
            Self::Value => COLUMN_NAMES[1],
        }
    }
    fn width(&self) -> u16 {
        match self {
            Self::Key => 30,
            Self::Value => 50,
        }
    }
    fn render(&self, tag: &T) -> (String, Style) {
        (
            match self {
                Self::Key => tag.key(),
                Self::Value => tag.value(),
            }
            .into(),
            Style::default(),
        )
    }
}
pub fn columns<T: Tag + 'static>() -> Vec<Box<dyn Column<T>>> {
    vec![Box::new(TagColumn::Key), Box::new(TagColumn::Value)]
}

pub struct TagsConfig<'a, T> {
    pub table: &'a TableState<T>,
    pub items: &'a [T],
    pub columns: Vec<Box<dyn Column<T>>>,
    pub sort: TagSort,
    pub window: TableWindow,
    pub focus: InputFocus,
    pub mode: Mode,
    pub loading: bool,
}

pub fn render<T: Tag>(frame: &mut Frame, area: Rect, config: TagsConfig<'_, T>) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let view = config.table.prepare_view(
        filtered(config.items, &config.table.filter, config.sort),
        config.window,
    );
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &config.table.filter,
            placeholder: "Search tags",
            pagination: &pagination,
            mode: config.mode,
            is_input_focused: config.focus == InputFocus::Filter,
            is_pagination_focused: config.focus == InputFocus::Pagination,
        },
    );
    if config.loading {
        frame.render_widget(
            Paragraph::new("Loading tags...")
                .block(titled_block("Tags"))
                .alignment(Alignment::Center),
            chunks[1],
        );
        return;
    }
    let sort_column = match config.sort {
        TagSort::Source => "",
        TagSort::Key => "Key",
        TagSort::Value => "Value",
    };
    render_table(
        frame,
        TableConfig {
            items: view.items,
            selected_index: view.selected_index,
            expanded_index: view.expanded_index,
            columns: &config.columns,
            sort_column,
            sort_direction: SortDirection::Asc,
            title: format_title(&format!("Tags ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|tag| expanded_from_columns(&config.columns, tag))),
            is_active: config.mode == Mode::Normal,
        },
    );
}

pub fn fixed_columns() -> Vec<PreferenceColumn> {
    COLUMN_NAMES
        .into_iter()
        .map(|name| PreferenceColumn {
            id: name.into(),
            label: name.into(),
            visible: true,
            locked: true,
        })
        .collect()
}
pub fn preferences(columns: Vec<PreferenceColumn>, size: PageSize, small: bool) -> Preferences {
    Preferences::table_with_page_sizes(
        columns,
        size,
        if small {
            SMALL_PAGE_SIZES
        } else {
            PAGE_SIZE_OPTIONS
        },
    )
}

#[derive(Clone, Copy)]
pub(crate) enum Source {
    Efs,
    Fsx,
    DynamoDb,
    Ec2,
    IamUser,
    IamRole,
}
impl Source {
    pub(crate) fn active(app: &App) -> Option<Self> {
        use crate::ui::{dynamodb, ec2, efs, fsx, iam};
        match app.current_service {
            Service::EfsFileSystems
                if app.efs_state.current_file_system.is_some()
                    && app.efs_state.detail_tab == efs::DetailTab::Tags =>
            {
                Some(Self::Efs)
            }
            Service::FsxFileSystems
                if app.fsx_state.current_file_system.is_some()
                    && app.fsx_state.detail_tab == fsx::DetailTab::Tags =>
            {
                Some(Self::Fsx)
            }
            Service::DynamoDbTables
                if app.dynamodb_state.current_table.is_some()
                    && app.dynamodb_state.detail_tab == dynamodb::DetailTab::Tags =>
            {
                Some(Self::DynamoDb)
            }
            Service::Ec2Instances
                if app.ec2_state.current_instance.is_some()
                    && app.ec2_state.detail_tab == ec2::DetailTab::Tags =>
            {
                Some(Self::Ec2)
            }
            Service::IamUsers
                if app.iam_state.current_user.is_some()
                    && app.iam_state.user_tab == iam::UserTab::Tags =>
            {
                Some(Self::IamUser)
            }
            Service::IamRoles
                if app.iam_state.current_role.is_some()
                    && app.iam_state.role_tab == iam::RoleTab::Tags =>
            {
                Some(Self::IamRole)
            }
            _ => None,
        }
    }
    fn count(self, app: &App) -> usize {
        match self {
            Self::Efs => app
                .efs_state
                .file_systems
                .items
                .iter()
                .find(|fs| Some(&fs.file_system_id) == app.efs_state.current_file_system.as_ref())
                .map(|fs| filtered(&fs.tags, &app.efs_state.tags_table.filter, TagSort::Key).len())
                .unwrap_or(0),
            Self::Fsx => app
                .fsx_state
                .file_systems
                .items
                .iter()
                .find(|fs| Some(&fs.file_system_id) == app.fsx_state.current_file_system.as_ref())
                .map(|fs| filtered(&fs.tags, &app.fsx_state.tags_table.filter, TagSort::Key).len())
                .unwrap_or(0),
            Self::DynamoDb => filtered(
                &app.dynamodb_state.tags_table.items,
                &app.dynamodb_state.tags_table.filter,
                TagSort::Key,
            )
            .len(),
            Self::Ec2 => filtered(
                &app.ec2_state.tags.items,
                &app.ec2_state.tags.filter,
                TagSort::Value,
            )
            .len(),
            Self::IamUser => filtered(
                &app.iam_state.user_tags.items,
                &app.iam_state.user_tags.filter,
                TagSort::Source,
            )
            .len(),
            Self::IamRole => filtered(
                &app.iam_state.tags.items,
                &app.iam_state.tags.filter,
                TagSort::Source,
            )
            .len(),
        }
    }
}

pub fn active_filter_mut(app: &mut App) -> Option<&mut String> {
    Some(match Source::active(app)? {
        Source::Efs => &mut app.efs_state.tags_table.filter,
        Source::Fsx => &mut app.fsx_state.tags_table.filter,
        Source::DynamoDb => &mut app.dynamodb_state.tags_table.filter,
        Source::Ec2 => &mut app.ec2_state.tags.filter,
        Source::IamUser => &mut app.iam_state.user_tags.filter,
        Source::IamRole => &mut app.iam_state.tags.filter,
    })
}

pub fn reset_active(app: &mut App) -> bool {
    match Source::active(app) {
        Some(Source::Efs) => app.efs_state.tags_table.reset(),
        Some(Source::Fsx) => app.fsx_state.tags_table.reset(),
        Some(Source::DynamoDb) => app.dynamodb_state.tags_table.reset(),
        Some(Source::Ec2) => app.ec2_state.tags.reset(),
        Some(Source::IamUser) => app.iam_state.user_tags.reset(),
        Some(Source::IamRole) => app.iam_state.tags.reset(),
        None => return false,
    }
    true
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected {
        return false;
    }
    let Some(source) = Source::active(app) else {
        return false;
    };
    // Resource-level navigation, copying and reloads stay with their service.
    if !matches!(app.mode, Mode::Normal | Mode::FilterInput)
        || matches!(
            action,
            Action::GoBack
                | Action::Refresh
                | Action::Yank
                | Action::NextDetailTab
                | Action::PrevDetailTab
                | Action::ToggleExactMatch
                | Action::ToggleShowExpired
                | Action::CycleSortColumn
                | Action::ToggleSortDirection
        )
    {
        return false;
    }
    let count = source.count(app);
    macro_rules! dispatch {
        ($table:expr, $focus:expr) => {{
            if app.mode == Mode::Normal
                && !app.page_input.is_empty()
                && matches!(action, Action::ApplyFilter | Action::OpenColumnSelector)
            {
                if let Ok(page) = app.page_input.parse() {
                    inventory::go_to_page(&mut $table, page, count);
                }
                app.page_input.clear();
                return true;
            }
            inventory::handle_action(
                InventoryInput {
                    table: &mut $table,
                    focus: &mut $focus,
                    mode: &mut app.mode,
                    page_input: &mut app.page_input,
                    error_message: &mut app.error_message,
                    error_scroll: &mut app.error_scroll,
                    filtered_count: count,
                    selected_arn: None,
                    clipboard: &app.clipboard,
                },
                action,
            )
        }};
    }
    match source {
        Source::Efs => dispatch!(app.efs_state.tags_table, app.efs_state.input_focus),
        Source::Fsx => dispatch!(app.fsx_state.tags_table, app.fsx_state.input_focus),
        Source::DynamoDb => dispatch!(
            app.dynamodb_state.tags_table,
            app.dynamodb_state.input_focus
        ),
        Source::Ec2 => dispatch!(app.ec2_state.tags, app.ec2_state.input_focus),
        Source::IamUser => dispatch!(app.iam_state.user_tags, app.iam_state.user_input_focus),
        Source::IamRole => dispatch!(app.iam_state.tags, app.iam_state.role_input_focus),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::{bindings, PreferenceRow};
    use ratatui::{backend::TestBackend, Terminal};

    fn rows() -> Vec<(String, String)> {
        (0..25)
            .rev()
            .map(|i| (format!("key{i:02}"), format!("value{i:02}")))
            .collect()
    }

    #[test]
    fn tags_filter_key_or_value_and_sort_before_preparing_pages() {
        let rows = rows();
        assert_eq!(filtered(&rows, "KEY12", TagSort::Key)[0].1, "value12");
        assert_eq!(filtered(&rows, "VALUE12", TagSort::Value)[0].0, "key12");
        let typed = [crate::iam::RoleTag {
            key: "環境".into(),
            value: "PRODUCTION".into(),
        }];
        assert_eq!(filtered(&typed, "production", TagSort::Source).len(), 1);
        let state = TableState {
            page_size: PageSize::Ten,
            selected: 12,
            expanded_item: Some(11),
            ..TableState::new()
        };
        let view = state.prepare_view(
            filtered(&rows, "key", TagSort::Key),
            TableWindow::SelectedPage,
        );
        assert_eq!(view.items[0].0, "key10");
        assert_eq!(view.items.len(), 10);
        assert_eq!(view.selected_index, 2);
        assert_eq!(view.expanded_index, Some(1));
    }

    #[test]
    fn tags_renderer_applies_page_size_and_expansion_to_tuple_rows() {
        let rows = rows();
        let state = TableState {
            page_size: PageSize::Ten,
            selected: 12,
            expanded_item: Some(11),
            ..TableState::new()
        };
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    TagsConfig {
                        table: &state,
                        items: &rows,
                        columns: columns(),
                        sort: TagSort::Key,
                        window: TableWindow::SelectedPage,
                        focus: InputFocus::Filter,
                        mode: Mode::Normal,
                        loading: false,
                    },
                )
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Tags (25)"));
        assert!(text.contains("1 [2] 3"));
        assert!(text.contains("► key12"));
        assert!(text.contains("▼ key11"));
        assert!(text.contains("Value: value11"));
        assert!(!text.contains("key09"));
        assert!(!text.contains("key20"));
    }

    #[test]
    fn tags_actions_do_not_change_parent_inventory_and_preferences_match_page_options() {
        let mut app = App::new_without_client("test".into(), None);
        app.current_service = Service::FsxFileSystems;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.fsx_state.current_file_system = Some("fs".into());
        app.fsx_state.detail_tab = crate::ui::fsx::DetailTab::Tags;
        app.fsx_state.file_systems.items = vec![rusticity_core::fsx::FsxFileSystem {
            file_system_id: "fs".into(),
            tags: rows(),
            ..Default::default()
        }];
        app.fsx_state.tags_table.page_size = PageSize::Ten;
        app.handle_action(Action::PageDown);
        assert_eq!(app.fsx_state.tags_table.selected, 10);
        assert_eq!(app.fsx_state.file_systems.selected, 0);
        app.handle_action(Action::StartFilter);
        for c in "VALUE12".chars() {
            app.handle_action(Action::FilterInput(c));
        }
        assert_eq!(Source::active(&app).unwrap().count(&app), 1);
        assert_eq!(app.fsx_state.tags_table.selected, 0);
        assert!(app.fsx_state.file_systems.filter.is_empty());
        app.handle_action(Action::ApplyFilter);
        app.handle_action(Action::Select);
        assert_eq!(app.fsx_state.tags_table.expanded_item, Some(0));
        app.handle_action(Action::OpenColumnSelector);
        let model = bindings::active(&app).unwrap();
        assert_eq!(model.column_count(), 2);
        assert_eq!(
            model
                .rows()
                .iter()
                .filter(|r| matches!(r, PreferenceRow::PageSize { .. }))
                .count(),
            4
        );
        app.current_service = Service::IamRoles;
        app.iam_state.current_role = Some("role".into());
        app.iam_state.role_tab = crate::ui::iam::RoleTab::Tags;
        let model = bindings::active(&app).unwrap();
        assert_eq!(
            model
                .rows()
                .iter()
                .filter(|r| matches!(r, PreferenceRow::PageSize { .. }))
                .count(),
            3
        );
        assert_eq!(model.max_index(), 7);
    }

    #[test]
    fn tags_ec2_preferences_keep_custom_column_toggles() {
        let mut app = App::new_without_client("test".into(), None);
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.ec2_state.current_instance = Some("i-test".into());
        app.ec2_state.detail_tab = crate::ui::ec2::DetailTab::Tags;
        app.mode = Mode::ColumnSelector;
        let before = app.ec2_state.tag_visible_column_ids.clone();
        let inventory = app.ec2_visible_column_ids.clone();
        app.column_selector_index = 2;
        app.handle_action(Action::ToggleColumn);
        assert_eq!(app.ec2_state.tag_visible_column_ids.len(), before.len() - 1);
        app.column_selector_index = 1;
        app.handle_action(Action::ToggleColumn);
        assert_eq!(app.ec2_state.tag_visible_column_ids.len(), 1);
        assert_eq!(app.ec2_visible_column_ids, inventory);
    }
}
