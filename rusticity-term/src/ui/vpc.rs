use crate::app::App;
use crate::common::{
    filter_by_fields, render_pagination_text, ColumnId, CyclicEnum, InputFocus, SortDirection,
};
use crate::keymap::Mode;
use crate::table::{TableState, TableWindow};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::format_title;
use crate::ui::loading::render_table;
use crate::ui::table::{expanded_from_columns, TableConfig};
use crate::vpc::columns::Column;
use ratatui::prelude::*;
use rusticity_core::vpc::Vpc;

pub struct State {
    pub vpcs: TableState<Vpc>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    pub(crate) inventory: crate::list_read::ListRead<Vpc>,
    /// VPC ID to filter to once the list read completes.
    pub(crate) pending_focus: Option<String>,
    /// Chosen in Preferences; see `crate::sort`.
    pub sort: crate::sort::SortOrder,
    // ── Detail view (read-only) ──
    pub current_vpc: Option<String>,
    pub detail_tab: DetailTab,
    pub detail: Option<Vpc>,
    pub tags: TableState<(String, String)>,
    /// Chosen in Preferences on the Tags tab; see `crate::sort`.
    pub tags_sort: crate::sort::SortOrder,
    pub detail_loading: bool,
    pub detail_error: Option<String>,
}

/// Detail sub-tabs in console ribbon order. Only tabs Rusticity fully reads
/// are listed; the console Resource map, Flow logs, Related resources and
/// Integrations tabs are left out until they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Cidrs,
    Tags,
}

impl crate::common::CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[Self::Cidrs, Self::Tags];
}

impl DetailTab {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cidrs => "CIDRs",
            Self::Tags => "Tags",
        }
    }
}
/// The table default order.
pub fn default_sort() -> crate::sort::SortOrder {
    crate::sort::SortOrder::new(Column::VpcId.id(), SortDirection::Asc)
}

/// The Tags tab default order: Key, ascending, as the console lists tags.
pub fn tags_default_sort() -> crate::sort::SortOrder {
    crate::sort::SortOrder::new("Key", SortDirection::Asc)
}

/// Read-only VPC detail: the Details pane, the console ribbon below it, and
/// the active tab's content under the ribbon.
fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    use crate::ui::render_tabs;

    let state = &app.vpc_state;

    // The Details pane stays on screen for every tab; the ribbon sits below
    // it and only the area under the ribbon changes per tab.
    let pane_fields: Vec<(&str, String)> = match &state.detail {
        Some(vpc) => {
            let is_default = match vpc.is_default {
                Some(true) => "Yes",
                Some(false) => "No",
                None => "-",
            };
            // Console order, limited to fields this read returns. DNS
            // hostnames and resolution, IPv6 pool, Network Address Usage
            // metrics, firewall coverage and Route 53 Resolver rule groups
            // need further reads and are left out until they exist.
            vec![
                ("Name", vpc.name.clone()),
                ("VPC ID", vpc.vpc_id.clone()),
                ("State", vpc.state.clone()),
                ("Main route table", vpc.main_route_table.clone()),
                ("Owner ID", vpc.owner_id.clone()),
                ("Main network ACL", vpc.main_network_acl.clone()),
                ("Encryption control ID", vpc.encryption_control_id.clone()),
                ("Block Public Access", vpc.block_public_access.clone()),
                ("Tenancy", vpc.tenancy.clone()),
                ("Default VPC", is_default.to_string()),
                (
                    "Encryption control mode",
                    vpc.encryption_control_mode.clone(),
                ),
                ("DHCP option set", vpc.dhcp_options_id.clone()),
                ("IPv4 CIDR", vpc.ipv4_cidrs.clone()),
                ("IPv6 CIDR", vpc.ipv6_cidrs.clone()),
                ("ARN", vpc.arn.clone()),
            ]
        }
        None => Vec::new(),
    };
    let pane_used = if state.detail.is_some() {
        crate::ui::summary::render_summary(frame, area, "Details", &pane_fields)
    } else {
        let text = state
            .detail_error
            .clone()
            .unwrap_or_else(|| "Not loaded".into());
        frame.render_widget(ratatui::widgets::Paragraph::new(text), area);
        area
    };

    let ribbon = Rect {
        y: pane_used.y + pane_used.height,
        height: 1.min(
            area.height
                .saturating_sub(pane_used.y + pane_used.height - area.y),
        ),
        ..area
    };
    let tabs: Vec<(&str, DetailTab)> = DetailTab::ALL
        .iter()
        .map(|tab| (tab.name(), *tab))
        .collect();
    render_tabs(frame, ribbon, &tabs, &state.detail_tab);

    let content = Rect {
        y: ribbon.y + ribbon.height,
        height: area
            .height
            .saturating_sub(ribbon.y + ribbon.height - area.y),
        ..area
    };
    match state.detail_tab {
        DetailTab::Tags => {
            use crate::tags::{self, TagSort, TagsConfig};
            // The order chosen in Preferences drives the shared renderer.
            let order = state
                .tags_sort
                .effective(tags_default_sort(), &["Key", "Value"]);
            let sort = if order.column == "Value" {
                TagSort::Value
            } else {
                TagSort::Key
            };
            let direction = order.direction;
            tags::render(
                frame,
                content,
                TagsConfig {
                    table: &state.tags,
                    items: &state.tags.items,
                    columns: tags::columns(),
                    sort,
                    direction,
                    window: crate::table::TableWindow::SelectedPage,
                    focus: state.input_focus,
                    mode: app.mode,
                    loading: state.detail_loading,
                },
            );
        }
        DetailTab::Cidrs => {
            if let Some(vpc) = &state.detail {
                let fields = [
                    ("IPv4 CIDR", vpc.ipv4_cidrs.clone()),
                    ("IPv6 CIDR", vpc.ipv6_cidrs.clone()),
                ];
                crate::ui::summary::render_summary(frame, content, "CIDRs", &fields);
            }
        }
    }
    if state.detail_loading && state.detail.is_some() {
        let body = Rect {
            x: content.x + 1,
            y: content.y + 1,
            width: content.width.saturating_sub(2),
            height: content.height.saturating_sub(2),
        };
        frame.render_widget(ratatui::widgets::Paragraph::new("Loading tags…"), body);
    } else if let Some(error) = &state.detail_error {
        let body = Rect {
            x: content.x + 1,
            y: content.y + 1,
            width: content.width.saturating_sub(2),
            height: content.height.saturating_sub(2),
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(error.clone())
                .style(ratatui::style::Style::default().fg(Color::Red)),
            body,
        );
    }
}

/// Header name and direction of the order in effect.
fn sort_header(state: &State) -> (String, SortDirection) {
    let order = state
        .sort
        .effective(default_sort(), &state.visible_column_ids);
    let name = Column::from_id(order.column)
        .map(|c| c.name())
        .unwrap_or_default();
    (name, order.direction)
}

impl State {
    pub(crate) fn load_status(&self) -> crate::async_read::ReadStatus {
        self.inventory.status()
    }
}
impl Default for State {
    fn default() -> Self {
        Self {
            vpcs: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
            inventory: Default::default(),
            pending_focus: None,
            sort: default_sort(),
            current_vpc: None,
            detail_tab: DetailTab::Cidrs,
            detail: None,
            tags: TableState::new(),
            tags_sort: tags_default_sort(),
            detail_loading: false,
            detail_error: None,
        }
    }
}

pub fn filtered_vpcs(app: &App) -> Vec<&Vpc> {
    let mut vpcs = filter_by_fields(
        &app.vpc_state.vpcs.items,
        &app.vpc_state.vpcs.filter,
        |vpc| {
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
        },
    );
    vpcs.sort_by(|a, b| a.vpc_id.cmp(&b.vpc_id));
    let order = app
        .vpc_state
        .sort
        .effective(default_sort(), &app.vpc_state.visible_column_ids);
    crate::sort::apply(&mut vpcs, order, |row, id| {
        Column::from_id(id)
            .map(|c| c.render(row).0)
            .unwrap_or_default()
    });
    vpcs
}

pub fn render_vpcs(frame: &mut Frame, app: &App, area: Rect) {
    if app.vpc_state.current_vpc.is_some() {
        render_detail(frame, app, area);
        return;
    }
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.vpc_state;
    let sort_header = sort_header(state);
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
            sort_column: &sort_header.0,
            sort_direction: sort_header.1,
            title: format_title(&format!("VPCs ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|vpc| {
                expanded_from_columns(&expanded_columns, vpc)
            })),
            is_active: app.mode == Mode::Normal,
        },
        state.inventory.status(),
        "VPCs",
        state.inventory.error(),
        crate::ui::loading::EmptyState {
            no_items: "No VPCs in this Region.",
            has_unfiltered_items: !state.vpcs.items.is_empty(),
        },
    );
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    fn vpc(name: &str, vpc_id: &str, state: &str) -> Vpc {
        Vpc {
            name: name.to_string(),
            vpc_id: vpc_id.to_string(),
            state: state.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn searches_many_fields_case_insensitively_and_keeps_vpc_id_sort() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.vpc_state.vpcs.items = vec![
            vpc("prod", "vpc-zzz", "available"),
            vpc("dev", "vpc-aaa", "available"),
        ];

        // Name match (case-insensitive).
        app.vpc_state.vpcs.filter = "PROD".to_string();
        let f = filtered_vpcs(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].vpc_id, "vpc-zzz");

        // Shared state match → both, sorted by vpc_id ascending.
        app.vpc_state.vpcs.filter = "available".to_string();
        let f = filtered_vpcs(&app);
        assert_eq!(
            f.iter().map(|v| v.vpc_id.as_str()).collect::<Vec<_>>(),
            ["vpc-aaa", "vpc-zzz"]
        );
    }

    #[test]
    fn empty_filter_returns_all_and_whitespace_is_significant() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.vpc_state.vpcs.items = vec![vpc("prod", "vpc-1", "available")];

        app.vpc_state.vpcs.filter = String::new();
        assert_eq!(filtered_vpcs(&app).len(), 1);

        // No trimming.
        app.vpc_state.vpcs.filter = " vpc-1 ".to_string();
        assert!(filtered_vpcs(&app).is_empty());
    }
}
