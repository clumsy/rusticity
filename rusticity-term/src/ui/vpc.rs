use crate::app::App;
use crate::common::{render_pagination_text, ColumnId, InputFocus, SortDirection};
use crate::keymap::Mode;
use crate::table::{TableState, TableWindow};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{expanded_from_columns, render_table, TableConfig};
use crate::ui::{format_title, titled_block};
use crate::vpc::columns::Column;
use ratatui::{prelude::*, widgets::Paragraph};
use rusticity_core::vpc::Vpc;

pub struct State {
    pub vpcs: TableState<Vpc>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            vpcs: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
        }
    }
}

pub fn filtered_vpcs(app: &App) -> Vec<&Vpc> {
    let query = app.vpc_state.vpcs.filter.to_lowercase();
    let mut vpcs = app.vpc_state.vpcs.filtered(|vpc| {
        [
            &vpc.name,
            &vpc.vpc_id,
            &vpc.state,
            &vpc.ipv4_cidrs,
            &vpc.ipv6_cidrs,
            &vpc.owner_id,
            &vpc.region,
            &vpc.arn,
            &vpc.dhcp_options_id,
            &vpc.tenancy,
            &vpc.encryption_control_id,
            &vpc.encryption_control_mode,
            &vpc.block_public_access,
            &vpc.main_route_table,
            &vpc.main_network_acl,
        ]
        .iter()
        .any(|value| value.to_lowercase().contains(&query))
    });
    vpcs.sort_by(|a, b| a.vpc_id.cmp(&b.vpc_id));
    vpcs
}

pub fn render_vpcs(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.vpc_state;
    let view = state
        .vpcs
        .prepare_view(filtered_vpcs(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &state.vpcs.filter,
            placeholder: "Search by name, VPC ID, CIDR, or owner",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            is_pagination_focused: state.input_focus == InputFocus::Pagination,
        },
    );
    if state.vpcs.loading && state.vpcs.items.is_empty() {
        frame.render_widget(
            Paragraph::new("Loading VPCs...")
                .block(titled_block("VPCs"))
                .alignment(Alignment::Center),
            chunks[1],
        );
        return;
    }
    let columns: Vec<_> = state
        .visible_column_ids
        .iter()
        .filter_map(|id| Column::from_id(id))
        .map(Column::into_table_column)
        .collect();
    let expanded_columns: Vec<_> = Column::ALL
        .into_iter()
        .map(Column::into_table_column)
        .collect();
    render_table(
        frame,
        TableConfig {
            items: view.items,
            selected_index: view.selected_index,
            expanded_index: view.expanded_index,
            columns: &columns,
            sort_column: &Column::VpcId.name(),
            sort_direction: SortDirection::Asc,
            title: format_title(&format!("VPCs ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|vpc| {
                expanded_from_columns(&expanded_columns, vpc)
            })),
            is_active: app.mode == Mode::Normal,
        },
    );
}
