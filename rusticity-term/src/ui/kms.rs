use crate::app::App;
use crate::common::{
    render_pagination_text, ColumnId, CyclicEnum, InputFocus, SortDirection, TextFilter,
};
use crate::kms::key::{self, Key as KmsKey};
use crate::table::{TableState, TableWindow};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use crate::ui::{format_title, render_tabs};
use ratatui::prelude::*;

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct State {
    pub detail: crate::kms::detail::State,
    pub keys: TableState<KmsKey>,
    pub tab: Tab,
    pub input_focus: InputFocus,
    /// Visible column IDs for the AWS managed keys tab
    pub aws_visible_column_ids: Vec<ColumnId>,
    /// Visible column IDs for the Customer managed keys tab
    pub customer_visible_column_ids: Vec<ColumnId>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        use crate::kms::key::Column;
        Self {
            detail: crate::kms::detail::State::default(),
            keys: TableState::new(),
            tab: Tab::AwsManaged,
            input_focus: InputFocus::Filter,
            aws_visible_column_ids: Column::aws_managed_default_visible().to_vec(),
            customer_visible_column_ids: Column::customer_managed_default_visible().to_vec(),
        }
    }

    /// Returns the currently active visible column IDs based on the tab.
    pub fn visible_column_ids(&self) -> &Vec<ColumnId> {
        match self.tab {
            Tab::AwsManaged => &self.aws_visible_column_ids,
            Tab::CustomerManaged => &self.customer_visible_column_ids,
        }
    }

    /// Returns the mutable currently active visible column IDs.
    pub fn visible_column_ids_mut(&mut self) -> &mut Vec<ColumnId> {
        match self.tab {
            Tab::AwsManaged => &mut self.aws_visible_column_ids,
            Tab::CustomerManaged => &mut self.customer_visible_column_ids,
        }
    }

    /// Returns the full column list for the current tab.
    pub fn tab_column_ids(&self) -> Vec<ColumnId> {
        use crate::kms::key::Column;
        match self.tab {
            Tab::AwsManaged => Column::aws_managed_columns()
                .iter()
                .map(|c| c.id())
                .collect(),
            Tab::CustomerManaged => Column::customer_managed_columns()
                .iter()
                .map(|c| c.id())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    AwsManaged,
    CustomerManaged,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[Self::AwsManaged, Self::CustomerManaged];
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::AwsManaged => "AWS managed keys",
            Tab::CustomerManaged => "Customer managed keys",
        }
    }
}

pub fn filtered_kms_keys(app: &App) -> Vec<&KmsKey> {
    let tab_filter: Box<dyn Fn(&&KmsKey) -> bool> = match app.kms_state.tab {
        Tab::AwsManaged => Box::new(|k: &&KmsKey| k.key_manager == "Aws"),
        Tab::CustomerManaged => Box::new(|k: &&KmsKey| k.key_manager == "Customer"),
    };

    let text_filter = TextFilter::new(&app.kms_state.keys.filter);
    app.kms_state
        .keys
        .items
        .iter()
        .filter(|k| tab_filter(k))
        .filter(|k| text_filter.matches_any([&k.alias, &k.key_id, &k.description]))
        .collect()
}

pub fn render_keys(frame: &mut Frame, app: &App, area: Rect) {
    if crate::kms::detail::active(app) {
        crate::kms::detail::render(frame, app, area);
        return;
    }
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
    render_tabs(frame, chunks[0], &tabs, &app.kms_state.tab);

    // Filtered keys for current tab
    let view = app
        .kms_state
        .keys
        .prepare_view(filtered_kms_keys(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);

    // Filter bar
    render_simple_filter(
        frame,
        chunks[1],
        SimpleFilterConfig {
            filter_text: &app.kms_state.keys.filter,
            placeholder: "Search by alias, key ID, or description",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.kms_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.kms_state.input_focus == InputFocus::Pagination,
        },
    );

    let title = format_title(&format!(
        "{} ({})",
        app.kms_state.tab.name(),
        view.total_items
    ));

    let columns: Vec<Box<dyn Column<KmsKey>>> = app
        .kms_state
        .visible_column_ids()
        .iter()
        .filter_map(|col_id| {
            key::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<KmsKey>>)
        })
        .collect();

    let config = TableConfig {
        items: view.items,
        selected_index: view.selected_index,
        expanded_index: view.expanded_index,
        columns: &columns,
        sort_column: "Alias",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[2],
        get_expanded_content: Some(Box::new(|k: &KmsKey| expanded_from_columns(&columns, k))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };

    render_table(frame, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_names() {
        assert_eq!(Tab::AwsManaged.name(), "AWS managed keys");
        assert_eq!(Tab::CustomerManaged.name(), "Customer managed keys");
    }

    #[test]
    fn test_tab_cycling() {
        assert_eq!(Tab::AwsManaged.next(), Tab::CustomerManaged);
        assert_eq!(Tab::CustomerManaged.next(), Tab::AwsManaged);
        assert_eq!(Tab::CustomerManaged.prev(), Tab::AwsManaged);
        assert_eq!(Tab::AwsManaged.prev(), Tab::CustomerManaged);
    }

    #[test]
    fn test_state_default_tab() {
        let state = State::new();
        assert_eq!(state.tab, Tab::AwsManaged);
    }

    #[test]
    fn test_filtered_kms_keys_aws_tab_only_returns_aws_keys() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.kms_state.tab = Tab::AwsManaged;
        app.kms_state.keys.items = vec![
            KmsKey {
                key_id: "aws-key".to_string(),
                key_arn: "arn:1".to_string(),
                alias: "aws/s3".to_string(),
                description: String::new(),
                key_state: "Enabled".to_string(),
                key_usage: "EncryptDecrypt".to_string(),
                key_spec: "SymmetricDefault".to_string(),
                key_manager: "Aws".to_string(),
                creation_date: String::new(),
                expiration_date: String::new(),
                deletion_date: String::new(),
                custom_key_store_id: String::new(),
                origin: "AwsKms".to_string(),
                multi_region: false,
                enabled: true,
            },
            KmsKey {
                key_id: "cust-key".to_string(),
                key_arn: "arn:2".to_string(),
                alias: "my-key".to_string(),
                description: String::new(),
                key_state: "Enabled".to_string(),
                key_usage: "EncryptDecrypt".to_string(),
                key_spec: "SymmetricDefault".to_string(),
                key_manager: "Customer".to_string(),
                creation_date: String::new(),
                expiration_date: String::new(),
                deletion_date: String::new(),
                custom_key_store_id: String::new(),
                origin: "AwsKms".to_string(),
                multi_region: false,
                enabled: true,
            },
        ];

        let filtered = filtered_kms_keys(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key_id, "aws-key");
    }

    #[test]
    fn test_filtered_kms_keys_customer_tab() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.kms_state.tab = Tab::CustomerManaged;
        app.kms_state.keys.items = vec![
            KmsKey {
                key_id: "aws-key".to_string(),
                key_arn: String::new(),
                alias: "aws/s3".to_string(),
                description: String::new(),
                key_state: "Enabled".to_string(),
                key_usage: String::new(),
                key_spec: String::new(),
                key_manager: "Aws".to_string(),
                creation_date: String::new(),
                origin: String::new(),
                expiration_date: String::new(),
                deletion_date: String::new(),
                custom_key_store_id: String::new(),
                multi_region: false,
                enabled: true,
            },
            KmsKey {
                key_id: "cust-key".to_string(),
                key_arn: String::new(),
                alias: "my-key".to_string(),
                description: String::new(),
                key_state: "Enabled".to_string(),
                key_usage: String::new(),
                key_spec: String::new(),
                key_manager: "Customer".to_string(),
                creation_date: String::new(),
                expiration_date: String::new(),
                deletion_date: String::new(),
                custom_key_store_id: String::new(),
                origin: String::new(),
                multi_region: false,
                enabled: true,
            },
        ];

        let filtered = filtered_kms_keys(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key_id, "cust-key");
    }

    #[test]
    fn test_filtered_kms_keys_text_filter() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.kms_state.tab = Tab::CustomerManaged;
        app.kms_state.keys.filter = "prod".to_string();
        app.kms_state.keys.items = vec![
            KmsKey {
                key_id: "key1".to_string(),
                key_arn: String::new(),
                alias: "prod-key".to_string(),
                description: String::new(),
                key_state: "Enabled".to_string(),
                key_usage: String::new(),
                key_spec: String::new(),
                key_manager: "Customer".to_string(),
                creation_date: String::new(),
                expiration_date: String::new(),
                deletion_date: String::new(),
                custom_key_store_id: String::new(),
                origin: String::new(),
                multi_region: false,
                enabled: true,
            },
            KmsKey {
                key_id: "key2".to_string(),
                key_arn: String::new(),
                alias: "dev-key".to_string(),
                description: String::new(),
                key_state: "Enabled".to_string(),
                key_usage: String::new(),
                key_spec: String::new(),
                key_manager: "Customer".to_string(),
                creation_date: String::new(),
                expiration_date: String::new(),
                deletion_date: String::new(),
                custom_key_store_id: String::new(),
                origin: String::new(),
                multi_region: false,
                enabled: true,
            },
        ];

        let filtered = filtered_kms_keys(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].alias, "prod-key");
    }

    fn key(key_id: &str, alias: &str, manager: &str) -> KmsKey {
        KmsKey {
            key_id: key_id.to_string(),
            key_arn: String::new(),
            alias: alias.to_string(),
            description: String::new(),
            key_state: "Enabled".to_string(),
            key_usage: String::new(),
            key_spec: String::new(),
            key_manager: manager.to_string(),
            creation_date: String::new(),
            expiration_date: String::new(),
            deletion_date: String::new(),
            custom_key_store_id: String::new(),
            origin: String::new(),
            multi_region: false,
            enabled: true,
        }
    }

    #[test]
    fn test_text_filter_combined_with_tab_predicate() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.kms_state.tab = Tab::CustomerManaged;
        app.kms_state.keys.filter = "prod".to_string();
        app.kms_state.keys.items = vec![
            key("k1", "prod-aws", "Aws"),       // text matches but wrong tab
            key("k2", "prod-cust", "Customer"), // text + tab both match
            key("k3", "dev-cust", "Customer"),  // tab matches, text does not
        ];
        let filtered = filtered_kms_keys(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key_id, "k2");
    }

    #[test]
    fn test_text_filter_unicode_case_insensitive_and_no_trim() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.kms_state.tab = Tab::CustomerManaged;
        app.kms_state.keys.items = vec![key("k1", "café-key", "Customer")];

        app.kms_state.keys.filter = "CAFÉ".to_string();
        assert_eq!(filtered_kms_keys(&app).len(), 1);

        // Whitespace is significant — padded query must not match.
        app.kms_state.keys.filter = " café-key ".to_string();
        assert!(filtered_kms_keys(&app).is_empty());

        // Empty filter keeps every row on the tab.
        app.kms_state.keys.filter = String::new();
        assert_eq!(filtered_kms_keys(&app).len(), 1);
    }
}
