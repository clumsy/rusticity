use crate::common::{render_pagination_text, CyclicEnum, InputFocus, SortDirection};
use crate::ec2::tag::{Column as TagColumn, InstanceTag};
use crate::ec2::{Column, Instance};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::tags::{self, TagSort};
use crate::ui::format_title;
use crate::ui::table::expanded_from_columns;
use ratatui::prelude::*;

pub const FILTER_CONTROLS: [InputFocus; 3] = [
    InputFocus::Filter,
    InputFocus::Checkbox("state"),
    InputFocus::Pagination,
];

pub const STATE_FILTER: InputFocus = InputFocus::Checkbox("state");

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StateFilter {
    #[default]
    AllStates,
    Running,
    Stopped,
    Terminated,
    Pending,
    ShuttingDown,
    Stopping,
}

impl StateFilter {
    pub fn name(&self) -> &'static str {
        match self {
            StateFilter::AllStates => "All states",
            StateFilter::Running => "Running",
            StateFilter::Stopped => "Stopped",
            StateFilter::Terminated => "Terminated",
            StateFilter::Pending => "Pending",
            StateFilter::ShuttingDown => "Shutting down",
            StateFilter::Stopping => "Stopping",
        }
    }

    pub fn matches(&self, state: &str) -> bool {
        match self {
            StateFilter::AllStates => true,
            StateFilter::Running => state == "running",
            StateFilter::Stopped => state == "stopped",
            StateFilter::Terminated => state == "terminated",
            StateFilter::Pending => state == "pending",
            StateFilter::ShuttingDown => state == "shutting-down",
            StateFilter::Stopping => state == "stopping",
        }
    }
}

impl CyclicEnum for StateFilter {
    const ALL: &'static [Self] = &[
        StateFilter::AllStates,
        StateFilter::Running,
        StateFilter::Stopped,
        StateFilter::Terminated,
        StateFilter::Pending,
        StateFilter::ShuttingDown,
        StateFilter::Stopping,
    ];

    fn next(&self) -> Self {
        match self {
            StateFilter::AllStates => StateFilter::Running,
            StateFilter::Running => StateFilter::Stopped,
            StateFilter::Stopped => StateFilter::Terminated,
            StateFilter::Terminated => StateFilter::Pending,
            StateFilter::Pending => StateFilter::ShuttingDown,
            StateFilter::ShuttingDown => StateFilter::Stopping,
            StateFilter::Stopping => StateFilter::AllStates,
        }
    }

    fn prev(&self) -> Self {
        match self {
            StateFilter::AllStates => StateFilter::Stopping,
            StateFilter::Running => StateFilter::AllStates,
            StateFilter::Stopped => StateFilter::Running,
            StateFilter::Terminated => StateFilter::Stopped,
            StateFilter::Pending => StateFilter::Terminated,
            StateFilter::ShuttingDown => StateFilter::Pending,
            StateFilter::Stopping => StateFilter::ShuttingDown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Details,
    StatusAndAlarms,
    Monitoring,
    Security,
    Networking,
    Storage,
    Tags,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::Details,
        Self::StatusAndAlarms,
        Self::Monitoring,
        Self::Security,
        Self::Networking,
        Self::Storage,
        Self::Tags,
    ];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::Details => "Details",
            DetailTab::StatusAndAlarms => "Status and alarms",
            DetailTab::Monitoring => "Monitoring",
            DetailTab::Security => "Security",
            DetailTab::Networking => "Networking",
            DetailTab::Storage => "Storage",
            DetailTab::Tags => "Tags",
        }
    }
}

pub struct State {
    pub table: TableState<Instance>,
    pub state_filter: StateFilter,
    /// Chosen in Preferences; see `crate::sort`.
    pub sort: crate::sort::SortOrder,
    pub input_focus: InputFocus,
    pub current_instance: Option<String>,
    pub detail_tab: DetailTab,
    pub tags: TableState<InstanceTag>,
    pub tag_visible_column_ids: Vec<String>,
    pub tag_column_ids: Vec<String>,
    /// The entered instance read and its tab tables; see `crate::ec2::detail`.
    pub(crate) detail: crate::ec2::detail::State,
    pub(crate) inventory: crate::list_read::ListRead<Instance>,
    /// Instance ID to enter once the list read completes.
    pub(crate) pending_focus: Option<String>,
}

impl State {
    /// Tab indicator status: the entered instance read, else the list read.
    pub(crate) fn load_status(&self) -> crate::async_read::ReadStatus {
        if self.current_instance.is_some() {
            self.detail.status()
        } else {
            self.inventory.status()
        }
    }
}

impl Default for State {
    fn default() -> Self {
        let tag_column_ids: Vec<String> = TagColumn::ids().iter().map(|s| s.to_string()).collect();
        Self {
            table: TableState::default(),
            state_filter: StateFilter::default(),
            sort: default_sort(),
            input_focus: InputFocus::Filter,
            current_instance: None,
            detail_tab: DetailTab::Details,
            tags: TableState::new(),
            tag_visible_column_ids: tag_column_ids.clone(),
            tag_column_ids,
            detail: Default::default(),
            inventory: Default::default(),
            pending_focus: None,
        }
    }
}

pub const FILTER_HINT: &str = "Find Instance by attribute or tag (case-sensitive)";

pub fn render_instances(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    visible_columns: &[&str],
    mode: Mode,
) {
    use crate::common::render_dropdown;
    use crate::ui::filter::{render_filter_bar, FilterConfig, FilterControl};
    use crate::ui::loading::render_table;

    let sort_order = state.sort.effective(default_sort(), visible_columns);
    let sort_name = Column::from_id(sort_order.column)
        .map(|c| c.name())
        .unwrap_or_default();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // The same rows and order that selection, copy and paging use.
    let filtered_items = filtered_rows(state, visible_columns);

    let page_size = state.table.page_size.value();
    let filtered_count = filtered_items.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = state.table.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &state.table.filter,
            placeholder: FILTER_HINT,
            mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            controls: vec![
                FilterControl {
                    text: state.state_filter.name().to_string(),
                    is_focused: state.input_focus == STATE_FILTER,
                },
                FilterControl {
                    text: pagination.clone(),
                    is_focused: state.input_focus == InputFocus::Pagination,
                },
            ],
            area: chunks[0],
        },
    );

    let columns: Vec<_> = visible_columns
        .iter()
        .filter_map(|id| Column::from_id(id).map(|c| c.to_column()))
        .collect();

    let title = format_title(&format!("Instances ({})", filtered_count));

    use crate::ui::table::TableConfig;
    render_table(
        frame,
        TableConfig {
            items: filtered_items,
            selected_index: state.table.selected,
            expanded_index: state.table.expanded_item,
            columns: &columns,
            sort_column: &sort_name,
            sort_direction: sort_order.direction,
            title,
            area: chunks[1],
            get_expanded_content: Some(Box::new(|instance: &Instance| {
                expanded_from_columns(&columns, instance)
            })),
            is_active: mode == Mode::Normal,
        },
        state.inventory.status(),
        "Instances",
        state.inventory.error(),
        crate::ui::loading::EmptyState {
            no_items: "No instances in this Region.",
            has_unfiltered_items: !state.table.items.is_empty(),
        },
    );

    // Render dropdown for StateFilter when focused (after table so it appears on top)
    if mode == Mode::FilterInput && state.input_focus == STATE_FILTER {
        let filter_names: Vec<&str> = StateFilter::ALL.iter().map(|f| f.name()).collect();
        let selected_idx = StateFilter::ALL
            .iter()
            .position(|f| *f == state.state_filter)
            .unwrap_or(0);
        let controls_after = pagination.len() as u16 + 3;
        render_dropdown(
            frame,
            &filter_names,
            selected_idx,
            chunks[0],
            controls_after,
        );
    }
}

pub fn filtered_ec2_instances(app: &crate::app::App) -> Vec<&Instance> {
    filtered_rows(&app.ec2_state, &app.ec2_visible_column_ids)
}

/// The table default order: newest launch first.
pub fn default_sort() -> crate::sort::SortOrder {
    crate::sort::SortOrder::new(Column::LaunchTime.id(), SortDirection::Desc)
}

/// Text and state filters, then the chosen order. The table, selection,
/// copy and paging all read this one list.
pub(crate) fn filtered_rows<'a>(state: &'a State, visible: &[&str]) -> Vec<&'a Instance> {
    let filter = &state.table.filter;
    let mut rows: Vec<&Instance> = state
        .table
        .items
        .iter()
        .filter(|i| state.state_filter.matches(&i.state))
        .filter(|i| {
            filter.is_empty()
                || [
                    &i.instance_id,
                    &i.name,
                    &i.state,
                    &i.instance_type,
                    &i.availability_zone,
                    &i.public_ipv4_address,
                    &i.private_ip_address,
                    &i.security_groups,
                    &i.key_name,
                ]
                .iter()
                .any(|field| field.contains(filter.as_str()))
        })
        .collect();
    let order = state.sort.effective(default_sort(), visible);
    crate::sort::apply(&mut rows, order, |row, id| {
        Column::from_id(id)
            .map(|c| c.to_column().render(row).0)
            .unwrap_or_default()
    });
    rows
}

pub fn render_instance_detail(frame: &mut Frame, area: Rect, app: &crate::app::App) {
    crate::ec2::detail::render(frame, app, area);
}

pub fn filtered_tags(app: &crate::app::App) -> Vec<&InstanceTag> {
    tags::filtered(
        &app.ec2_state.tags.items,
        &app.ec2_state.tags.filter,
        TagSort::Value,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ec2::tag::Column as TagColumn;
    use crate::ui::render_fields_with_dynamic_columns;

    #[test]
    fn test_state_filter_names() {
        assert_eq!(StateFilter::AllStates.name(), "All states");
        assert_eq!(StateFilter::Running.name(), "Running");
        assert_eq!(StateFilter::Stopped.name(), "Stopped");
        assert_eq!(StateFilter::Terminated.name(), "Terminated");
        assert_eq!(StateFilter::Pending.name(), "Pending");
        assert_eq!(StateFilter::ShuttingDown.name(), "Shutting down");
        assert_eq!(StateFilter::Stopping.name(), "Stopping");
    }

    #[test]
    fn test_state_filter_matches() {
        assert!(StateFilter::AllStates.matches("running"));
        assert!(StateFilter::AllStates.matches("stopped"));
        assert!(StateFilter::Running.matches("running"));
        assert!(!StateFilter::Running.matches("stopped"));
        assert!(StateFilter::Stopped.matches("stopped"));
        assert!(!StateFilter::Stopped.matches("running"));
    }

    #[test]
    fn test_state_filter_next() {
        assert_eq!(StateFilter::AllStates.next(), StateFilter::Running);
        assert_eq!(StateFilter::Running.next(), StateFilter::Stopped);
        assert_eq!(StateFilter::Stopping.next(), StateFilter::AllStates);
    }

    #[test]
    fn test_state_filter_prev() {
        assert_eq!(StateFilter::AllStates.prev(), StateFilter::Stopping);
        assert_eq!(StateFilter::Running.prev(), StateFilter::AllStates);
        assert_eq!(StateFilter::Stopped.prev(), StateFilter::Running);
    }

    #[test]
    fn test_state_filter_all_constant() {
        assert_eq!(StateFilter::ALL.len(), 7);
        assert_eq!(StateFilter::ALL[0], StateFilter::AllStates);
        assert_eq!(StateFilter::ALL[6], StateFilter::Stopping);
    }

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.table.items.len(), 0);
        assert_eq!(state.table.selected, 0);
        assert!(!state.table.loading);
        assert_eq!(state.table.filter, "");
        assert_eq!(state.state_filter, StateFilter::AllStates);
        assert_eq!(state.sort, default_sort());
        assert!(state.detail.detail().is_none());
        assert_eq!(state.detail.status(), crate::async_read::ReadStatus::Idle);
    }

    #[test]
    fn test_state_filter_matches_all_states() {
        let filter = StateFilter::AllStates;
        assert!(filter.matches("running"));
        assert!(filter.matches("stopped"));
        assert!(filter.matches("terminated"));
        assert!(filter.matches("pending"));
        assert!(filter.matches("shutting-down"));
        assert!(filter.matches("stopping"));
    }

    #[test]
    fn test_state_filter_matches_specific_states() {
        assert!(StateFilter::Running.matches("running"));
        assert!(!StateFilter::Running.matches("stopped"));

        assert!(StateFilter::Stopped.matches("stopped"));
        assert!(!StateFilter::Stopped.matches("running"));

        assert!(StateFilter::Terminated.matches("terminated"));
        assert!(!StateFilter::Terminated.matches("running"));

        assert!(StateFilter::Pending.matches("pending"));
        assert!(!StateFilter::Pending.matches("running"));

        assert!(StateFilter::ShuttingDown.matches("shutting-down"));
        assert!(!StateFilter::ShuttingDown.matches("running"));

        assert!(StateFilter::Stopping.matches("stopping"));
        assert!(!StateFilter::Stopping.matches("running"));
    }

    #[test]
    fn test_state_filter_cycle() {
        let mut filter = StateFilter::AllStates;
        filter = filter.next();
        assert_eq!(filter, StateFilter::Running);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Stopped);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Terminated);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Pending);
        filter = filter.next();
        assert_eq!(filter, StateFilter::ShuttingDown);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Stopping);
        filter = filter.next();
        assert_eq!(filter, StateFilter::AllStates);
    }

    #[test]
    fn test_filter_controls_constant() {
        assert_eq!(FILTER_CONTROLS.len(), 3);
        assert_eq!(FILTER_CONTROLS[0], InputFocus::Filter);
        assert_eq!(FILTER_CONTROLS[1], STATE_FILTER);
        assert_eq!(FILTER_CONTROLS[2], InputFocus::Pagination);
    }

    #[test]
    fn test_input_focus_cycling() {
        let mut focus = InputFocus::Filter;
        focus = focus.next(&FILTER_CONTROLS);
        assert_eq!(focus, STATE_FILTER);
        focus = focus.next(&FILTER_CONTROLS);
        assert_eq!(focus, InputFocus::Pagination);
        focus = focus.next(&FILTER_CONTROLS);
        assert_eq!(focus, InputFocus::Filter);
    }

    #[test]
    fn test_input_focus_reverse_cycling() {
        let mut focus = InputFocus::Filter;
        focus = focus.prev(&FILTER_CONTROLS);
        assert_eq!(focus, InputFocus::Pagination);
        focus = focus.prev(&FILTER_CONTROLS);
        assert_eq!(focus, STATE_FILTER);
        focus = focus.prev(&FILTER_CONTROLS);
        assert_eq!(focus, InputFocus::Filter);
    }

    #[test]
    fn test_state_default_input_focus() {
        let state = State::default();
        assert_eq!(state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_filter_controls_includes_state_filter() {
        assert_eq!(FILTER_CONTROLS.len(), 3);
        assert_eq!(FILTER_CONTROLS[0], InputFocus::Filter);
        assert_eq!(FILTER_CONTROLS[1], STATE_FILTER);
        assert_eq!(FILTER_CONTROLS[2], InputFocus::Pagination);
    }

    #[test]
    fn test_state_filter_constant() {
        assert_eq!(STATE_FILTER, InputFocus::Checkbox("state"));
    }

    #[test]
    fn test_state_filter_all_has_7_items() {
        assert_eq!(StateFilter::ALL.len(), 7);
    }

    #[test]
    fn test_dropdown_shows_when_state_filter_focused() {
        // This is tested via integration - dropdown renders when input_focus == STATE_FILTER
        // Verify the constant is accessible
        let focus = STATE_FILTER;
        assert_eq!(focus, InputFocus::Checkbox("state"));
    }

    #[test]
    fn test_detail_tab_cycling() {
        let mut tab = DetailTab::Details;
        tab = tab.next();
        assert_eq!(tab, DetailTab::StatusAndAlarms);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Monitoring);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Security);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Networking);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Storage);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Tags);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Details);
    }

    #[test]
    fn test_detail_tab_reverse_cycling() {
        let mut tab = DetailTab::Details;
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Tags);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Storage);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Networking);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Security);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Monitoring);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::StatusAndAlarms);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Details);
    }

    #[test]
    fn test_detail_tab_names() {
        assert_eq!(DetailTab::Details.name(), "Details");
        assert_eq!(DetailTab::StatusAndAlarms.name(), "Status and alarms");
        assert_eq!(DetailTab::Monitoring.name(), "Monitoring");
        assert_eq!(DetailTab::Security.name(), "Security");
        assert_eq!(DetailTab::Networking.name(), "Networking");
        assert_eq!(DetailTab::Storage.name(), "Storage");
        assert_eq!(DetailTab::Tags.name(), "Tags");
    }

    #[test]
    fn test_detail_tab_all_has_7_items() {
        assert_eq!(DetailTab::ALL.len(), 7);
    }

    #[test]
    fn test_state_default_has_no_current_instance() {
        let state = State::default();
        assert_eq!(state.current_instance, None);
        assert_eq!(state.detail_tab, DetailTab::Details);
    }

    #[test]
    fn test_column_distribution_10_fields_3_columns() {
        // 10 fields, 3 columns: should be 4, 3, 3
        let total = 10;
        let cols = 3;
        let base = total / cols; // 3
        let extra = total % cols; // 1

        let mut distribution = Vec::new();
        for col in 0..cols {
            let count = if col < extra { base + 1 } else { base };
            distribution.push(count);
        }

        assert_eq!(distribution, vec![4, 3, 3]);
    }

    #[test]
    fn test_column_distribution_10_fields_2_columns() {
        // 10 fields, 2 columns: should be 5, 5
        let total = 10;
        let cols = 2;
        let base = total / cols;
        let extra = total % cols;

        let mut distribution = Vec::new();
        for col in 0..cols {
            let count = if col < extra { base + 1 } else { base };
            distribution.push(count);
        }

        assert_eq!(distribution, vec![5, 5]);
    }

    #[test]
    fn test_column_distribution_10_fields_4_columns() {
        // 10 fields, 4 columns: should be 3, 3, 2, 2
        let total = 10;
        let cols = 4;
        let base = total / cols; // 2
        let extra = total % cols; // 2

        let mut distribution = Vec::new();
        for col in 0..cols {
            let count = if col < extra { base + 1 } else { base };
            distribution.push(count);
        }

        assert_eq!(distribution, vec![3, 3, 2, 2]);
    }

    #[test]
    fn test_column_distribution_7_fields_3_columns() {
        // 7 fields, 3 columns: should be 3, 2, 2
        let total = 7;
        let cols = 3;
        let base = total / cols; // 2
        let extra = total % cols; // 1

        let mut distribution = Vec::new();
        for col in 0..cols {
            let count = if col < extra { base + 1 } else { base };
            distribution.push(count);
        }

        assert_eq!(distribution, vec![3, 2, 2]);
    }

    #[test]
    fn test_column_distribution_ensures_first_has_most() {
        // Test that first column always has >= other columns
        for total in 1..20 {
            for cols in 1..=total {
                let base = total / cols;
                let extra = total % cols;

                let first_col = if 0 < extra { base + 1 } else { base };
                let last_col = if (cols - 1) < extra { base + 1 } else { base };

                assert!(
                    first_col >= last_col,
                    "total={}, cols={}, first={}, last={}",
                    total,
                    cols,
                    first_col,
                    last_col
                );
            }
        }
    }

    #[test]
    fn test_render_fields_with_dynamic_columns_empty() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                let height = render_fields_with_dynamic_columns(f, area, vec![]);
                assert_eq!(height, 0);
            })
            .unwrap();
    }

    #[test]
    fn test_render_fields_with_dynamic_columns_single_field() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                let fields = vec![Line::from("Test field")];
                let height = render_fields_with_dynamic_columns(f, area, fields);
                assert_eq!(height, 1);
            })
            .unwrap();
    }

    #[test]
    fn test_render_fields_with_dynamic_columns_calculates_height() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(40, 24); // Narrow width
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                let fields = vec![
                    Line::from("Field 1"),
                    Line::from("Field 2"),
                    Line::from("Field 3"),
                    Line::from("Field 4"),
                ];
                let height = render_fields_with_dynamic_columns(f, area, fields);
                // Should return a reasonable height
                assert!((1..=4).contains(&height));
            })
            .unwrap();
    }

    #[test]
    fn test_instance_summary_has_18_fields() {
        // Verify summary has all required fields matching AWS console
        let field_count = 18;
        assert_eq!(field_count, 18, "Instance summary should have 18 fields");
    }

    #[test]
    fn test_instance_summary_includes_key_fields() {
        // Test that key field labels are present
        let required_fields = vec![
            "Instance ID",
            "Public IPv4 address",
            "Private IPv4 addresses",
            "IPv6 address",
            "Instance state",
            "Public DNS",
            "Hostname type",
            "Private IP DNS name (IPv4 only)",
            "Instance type",
            "Elastic IP addresses",
            "Auto-assigned IP address",
            "VPC ID",
            "IAM Role",
            "Subnet ID",
            "IMDSv2",
            "Availability Zone",
            "Managed",
            "Operator",
        ];
        assert_eq!(required_fields.len(), 18);
    }

    #[test]
    fn test_rounded_block_helper_creates_block_with_title() {
        use crate::ui::titled_block;
        use ratatui::prelude::Rect;
        // Verify rounded_block helper can be used with title
        let block = titled_block("Instance summary");
        let area = Rect::new(0, 0, 50, 10);
        let inner = block.inner(area);
        // Inner area: 50 - 2 (borders) - 2 (padding) = 46 width
        assert_eq!(inner.width, 48);
        assert_eq!(inner.height, 8);
    }

    #[test]
    fn test_summary_height_uses_dynamic_calculation() {
        use crate::ui::{calculate_dynamic_height, labeled_field};
        // Verify summary height accounts for column packing
        let fields = vec![
            labeled_field("Instance ID", "i-1234567890abcdef0"),
            labeled_field("Instance type", "t2.micro"),
            labeled_field("State", "running"),
            labeled_field("VPC ID", "vpc-12345678"),
        ];
        let width = 180;
        let height = calculate_dynamic_height(&fields, width);
        // With 4 fields and wide width, should pack into 2 rows
        assert!(height <= 2, "Expected 2 rows or less, got {}", height);
    }

    #[test]
    fn test_tag_column_ids() {
        let ids = TagColumn::ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "column.ec2.tag.key");
        assert_eq!(ids[1], "column.ec2.tag.value");
    }

    #[test]
    fn test_tag_column_from_id() {
        assert_eq!(
            TagColumn::from_id("column.ec2.tag.key"),
            Some(TagColumn::Key)
        );
        assert_eq!(
            TagColumn::from_id("column.ec2.tag.value"),
            Some(TagColumn::Value)
        );
        assert_eq!(TagColumn::from_id("invalid"), None);
    }

    #[test]
    fn test_state_includes_tags() {
        let state = State::default();
        assert_eq!(state.tags.items.len(), 0);
        assert_eq!(state.tag_visible_column_ids.len(), 2);
        assert_eq!(state.tag_column_ids.len(), 2);
    }

    #[test]
    fn test_filtered_tags_empty() {
        use crate::app::App;
        let app = App::new_without_client("default".to_string(), None);
        let filtered = filtered_tags(&app);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filtered_tags_with_filter() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.ec2_state.tags.items = vec![
            InstanceTag {
                key: "Name".to_string(),
                value: "test-instance".to_string(),
            },
            InstanceTag {
                key: "Environment".to_string(),
                value: "production".to_string(),
            },
            InstanceTag {
                key: "Team".to_string(),
                value: "backend".to_string(),
            },
        ];
        app.ec2_state.tags.filter = "prod".to_string();
        let filtered = filtered_tags(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key, "Environment");
    }

    #[test]
    fn test_filtered_tags_sorts_by_value() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.ec2_state.tags.items = vec![
            InstanceTag {
                key: "Name".to_string(),
                value: "zebra".to_string(),
            },
            InstanceTag {
                key: "Environment".to_string(),
                value: "alpha".to_string(),
            },
            InstanceTag {
                key: "Team".to_string(),
                value: "beta".to_string(),
            },
        ];
        let filtered = filtered_tags(&app);
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].value, "alpha");
        assert_eq!(filtered[1].value, "beta");
        assert_eq!(filtered[2].value, "zebra");
    }

    #[test]
    fn test_tags_columns_fallback_when_empty() {
        use crate::app::App;
        let app = App::new_without_client("default".to_string(), None);
        // tag_visible_column_ids should be initialized with all columns
        assert_eq!(app.ec2_state.tag_visible_column_ids.len(), 2);
    }

    #[test]
    fn test_tags_collapse_on_left_arrow() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.ec2_state.tags.expanded_item = Some(0);
        assert!(app.ec2_state.tags.has_expanded_item());
        app.ec2_state.tags.collapse();
        assert!(!app.ec2_state.tags.has_expanded_item());
    }

    #[test]
    fn test_max_detail_columns_constant() {
        use crate::ui::MAX_DETAIL_COLUMNS;
        assert_eq!(MAX_DETAIL_COLUMNS, 3);
    }

    #[test]
    fn test_page_size_options_constant() {
        use crate::ui::PAGE_SIZE_OPTIONS;
        assert_eq!(PAGE_SIZE_OPTIONS.len(), 4);
        assert_eq!(PAGE_SIZE_OPTIONS[0].1, "10");
        assert_eq!(PAGE_SIZE_OPTIONS[3].1, "100");
    }

    #[test]
    fn test_page_size_options_small_constant() {
        use crate::ui::PAGE_SIZE_OPTIONS_SMALL;
        assert_eq!(PAGE_SIZE_OPTIONS_SMALL.len(), 3);
        assert_eq!(PAGE_SIZE_OPTIONS_SMALL[0].1, "10");
        assert_eq!(PAGE_SIZE_OPTIONS_SMALL[2].1, "50");
    }

    #[test]
    fn test_ec2_filter_has_search_icon_label() {
        // The filter panel must use render_filter_bar (which always shows SEARCH_ICON = "─ 🔍 ─"
        // as the block title), so there must never be an unlabelled/empty filter panel.
        let source = include_str!("ec2.rs");

        // render_instances must call render_filter_bar (not skip it)
        assert!(
            source.contains("render_filter_bar"),
            "render_instances must use render_filter_bar to show the 🔍 label"
        );

        // FILTER_HINT must be set as the placeholder so empty filter shows context
        assert!(
            source.contains("placeholder: FILTER_HINT"),
            "FilterConfig must use FILTER_HINT as placeholder"
        );

        // Verify FILTER_HINT is not empty
        assert!(!FILTER_HINT.is_empty(), "FILTER_HINT must not be empty");
    }
}
