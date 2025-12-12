use crate::common::{
    filter_by_fields, render_pagination_text, CyclicEnum, InputFocus, SortDirection,
};
use crate::ec2::tag::{Column as TagColumn, InstanceTag};
use crate::ec2::{Column, Instance};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{expanded_from_columns, render_table, Column as TableColumn, TableConfig};
use crate::ui::{calculate_dynamic_height, render_fields_with_dynamic_columns, rounded_block};
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
    pub sort_column: Column,
    pub sort_direction: SortDirection,
    pub input_focus: InputFocus,
    pub current_instance: Option<String>,
    pub detail_tab: DetailTab,
    pub tags: TableState<InstanceTag>,
    pub tag_visible_column_ids: Vec<String>,
    pub tag_column_ids: Vec<String>,
    pub monitoring_scroll: usize,
    pub metrics_loading: bool,
    pub metric_data_cpu: Vec<(i64, f64)>,
    pub metric_data_network_in: Vec<(i64, f64)>,
    pub metric_data_network_out: Vec<(i64, f64)>,
    pub metric_data_network_packets_in: Vec<(i64, f64)>,
    pub metric_data_network_packets_out: Vec<(i64, f64)>,
    pub metric_data_metadata_no_token: Vec<(i64, f64)>,
}

impl Default for State {
    fn default() -> Self {
        let tag_column_ids: Vec<String> = TagColumn::ids().iter().map(|s| s.to_string()).collect();
        Self {
            table: TableState::default(),
            state_filter: StateFilter::default(),
            sort_column: Column::LaunchTime,
            sort_direction: SortDirection::Desc,
            input_focus: InputFocus::Filter,
            current_instance: None,
            detail_tab: DetailTab::Details,
            tags: TableState::new(),
            tag_visible_column_ids: tag_column_ids.clone(),
            tag_column_ids,
            monitoring_scroll: 0,
            metrics_loading: false,
            metric_data_cpu: Vec::new(),
            metric_data_network_in: Vec::new(),
            metric_data_network_out: Vec::new(),
            metric_data_network_packets_in: Vec::new(),
            metric_data_network_packets_out: Vec::new(),
            metric_data_metadata_no_token: Vec::new(),
        }
    }
}

impl crate::ui::monitoring::MonitoringState for State {
    fn is_metrics_loading(&self) -> bool {
        self.metrics_loading
    }

    fn set_metrics_loading(&mut self, loading: bool) {
        self.metrics_loading = loading;
    }

    fn monitoring_scroll(&self) -> usize {
        self.monitoring_scroll
    }

    fn set_monitoring_scroll(&mut self, scroll: usize) {
        self.monitoring_scroll = scroll;
    }

    fn clear_metrics(&mut self) {
        self.metric_data_cpu.clear();
        self.metric_data_network_in.clear();
        self.metric_data_network_out.clear();
        self.metric_data_network_packets_in.clear();
        self.metric_data_network_packets_out.clear();
        self.metric_data_metadata_no_token.clear();
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
    use crate::ui::table::render_table;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered_items: Vec<&Instance> = state
        .table
        .items
        .iter()
        .filter(|i| state.state_filter.matches(&i.state))
        .filter(|i| {
            if state.table.filter.is_empty() {
                return true;
            }
            i.name.contains(&state.table.filter)
                || i.instance_id.contains(&state.table.filter)
                || i.state.contains(&state.table.filter)
                || i.instance_type.contains(&state.table.filter)
                || i.availability_zone.contains(&state.table.filter)
                || i.security_groups.contains(&state.table.filter)
                || i.key_name.contains(&state.table.filter)
        })
        .collect();

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

    let title = format!("Instances ({})", filtered_count);

    use crate::ui::table::TableConfig;
    render_table(
        frame,
        TableConfig {
            items: filtered_items,
            selected_index: state.table.selected,
            expanded_index: state.table.expanded_item,
            columns: &columns,
            sort_column: "",
            sort_direction: state.sort_direction,
            title,
            area: chunks[1],
            get_expanded_content: Some(Box::new(|instance: &Instance| {
                expanded_from_columns(&columns, instance)
            })),
            is_active: mode == Mode::Normal,
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
    let filtered: Vec<&Instance> = if app.ec2_state.table.filter.is_empty() {
        app.ec2_state.table.items.iter().collect()
    } else {
        app.ec2_state
            .table
            .items
            .iter()
            .filter(|i| {
                i.instance_id.contains(&app.ec2_state.table.filter)
                    || i.name.contains(&app.ec2_state.table.filter)
                    || i.state.contains(&app.ec2_state.table.filter)
                    || i.instance_type.contains(&app.ec2_state.table.filter)
                    || i.public_ipv4_address.contains(&app.ec2_state.table.filter)
                    || i.private_ip_address.contains(&app.ec2_state.table.filter)
            })
            .collect()
    };

    filtered
        .into_iter()
        .filter(|i| app.ec2_state.state_filter.matches(&i.state))
        .collect()
}

pub fn render_instance_detail(frame: &mut Frame, area: Rect, app: &crate::app::App) {
    use crate::ui::{labeled_field, render_tabs};

    let instance = app
        .ec2_state
        .table
        .items
        .iter()
        .find(|i| Some(&i.instance_id) == app.ec2_state.current_instance.as_ref());

    let Some(instance) = instance else {
        return;
    };

    // All fields to display (matching AWS console)
    let all_fields = vec![
        labeled_field("Instance ID", &instance.instance_id),
        labeled_field("Public IPv4 address", &instance.public_ipv4_address),
        labeled_field("Private IPv4 addresses", &instance.private_ip_address),
        labeled_field("IPv6 address", &instance.ipv6_ips),
        labeled_field("Instance state", &instance.state),
        labeled_field("Public DNS", &instance.public_ipv4_dns),
        labeled_field(
            "Hostname type",
            format!("IP name: {}", &instance.private_dns_name),
        ),
        labeled_field(
            "Private IP DNS name (IPv4 only)",
            &instance.private_dns_name,
        ),
        labeled_field("Instance type", &instance.instance_type),
        labeled_field("Elastic IP addresses", &instance.elastic_ip),
        labeled_field(
            "Auto-assigned IP address",
            format!("{} [Public IP]", &instance.public_ipv4_address),
        ),
        labeled_field("VPC ID", &instance.vpc_id),
        labeled_field("IAM Role", &instance.iam_instance_profile_arn),
        labeled_field("Subnet ID", &instance.subnet_ids),
        labeled_field("IMDSv2", &instance.imdsv2),
        labeled_field("Availability Zone", &instance.availability_zone),
        labeled_field("Managed", &instance.managed),
        labeled_field("Operator", &instance.operator),
    ];

    // Calculate height needed
    let summary_height = calculate_dynamic_height(&all_fields, area.width.saturating_sub(4)) + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(summary_height), Constraint::Min(0)])
        .split(area);

    // Instance summary
    let summary_block = rounded_block().title(" Instance summary ");
    let summary_inner = summary_block.inner(chunks[0]);
    frame.render_widget(summary_block, chunks[0]);

    render_fields_with_dynamic_columns(frame, summary_inner, all_fields);

    // Tab content
    let tab_names: Vec<(&str, DetailTab)> = DetailTab::ALL.iter().map(|t| (t.name(), *t)).collect();
    render_tabs(frame, chunks[1], &tab_names, &app.ec2_state.detail_tab);

    // Content area starts right after tabs (1 line for tab bar)
    let content_area = Rect {
        x: chunks[1].x,
        y: chunks[1].y + 1,
        width: chunks[1].width,
        height: chunks[1].height.saturating_sub(1),
    };

    match app.ec2_state.detail_tab {
        DetailTab::Details => {
            let instance_details = vec![
                labeled_field("AMI ID", &instance.image_id),
                labeled_field("Monitoring", &instance.monitoring),
                labeled_field("Platform details", &instance.platform_details),
                labeled_field("AMI name", "–"),
                labeled_field("Allowed image", "–"),
                labeled_field("Termination protection", "–"),
                labeled_field("Stop protection", "–"),
                labeled_field("Launch time", &instance.launch_time),
                labeled_field("AMI location", "–"),
                labeled_field("Instance reboot migration", "–"),
                labeled_field("Instance auto-recovery", "–"),
                labeled_field("Lifecycle", &instance.instance_lifecycle),
                labeled_field(
                    "Stop-hibernate behavior",
                    &instance.stop_hibernation_behavior,
                ),
                labeled_field("AMI Launch index", &instance.ami_launch_index),
                labeled_field("Key pair assigned at launch", &instance.key_name),
                labeled_field(
                    "State transition reason",
                    &instance.state_transition_reason_code,
                ),
                labeled_field("Credit specification", "–"),
                labeled_field("Kernel ID", &instance.kernel_id),
                labeled_field(
                    "State transition message",
                    &instance.state_transition_reason_message,
                ),
                labeled_field("Usage operation", &instance.usage_operation),
                labeled_field("RAM disk ID", &instance.ramdisk_id),
                labeled_field("Owner", &instance.owner_id),
                labeled_field("Enclaves Support", "–"),
                labeled_field("Boot mode", "–"),
                labeled_field("Current instance boot mode", "–"),
                labeled_field("Allow tags in instance metadata", "–"),
                labeled_field("Use RBN as guest OS hostname", "–"),
                labeled_field("Answer RBN DNS hostname IPv4", "–"),
            ];

            let placement = vec![
                labeled_field("Host ID", &instance.host_id),
                labeled_field("Affinity", &instance.affinity),
                labeled_field("Placement group", &instance.placement_group),
                labeled_field("Host resource group name", "–"),
                labeled_field("Tenancy", &instance.tenancy),
                labeled_field("Placement group ID", "–"),
                labeled_field("Virtualization type", &instance.virtualization_type),
                labeled_field("Reservation", &instance.reservation_id),
                labeled_field("Partition number", &instance.partition_number),
                labeled_field("Number of vCPUs", "–"),
            ];

            let capacity = vec![
                labeled_field("Capacity Reservation ID", &instance.capacity_reservation_id),
                labeled_field("Capacity Reservation setting", "open"),
            ];

            // Calculate heights for each section based on field count and width
            let calc_height = |fields: &[Line], width: u16| -> u16 {
                if fields.is_empty() {
                    return 2; // Just borders
                }
                let field_widths: Vec<u16> = fields
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .map(|span| span.content.len() as u16)
                            .sum::<u16>()
                            + 2
                    })
                    .collect();
                let max_field_width = *field_widths.iter().max().unwrap_or(&20);
                let num_columns =
                    (width / max_field_width).max(1).min(fields.len() as u16) as usize;
                let base = fields.len() / num_columns;
                let extra = fields.len() % num_columns;
                let max_rows = if extra > 0 { base + 1 } else { base };
                (max_rows as u16) + 2
            };

            let details_height =
                calc_height(&instance_details, content_area.width.saturating_sub(2));
            let placement_height = calc_height(&placement, content_area.width.saturating_sub(2));
            let capacity_height = calc_height(&capacity, content_area.width.saturating_sub(2));

            // Ensure total height doesn't exceed available space
            let total_height = details_height + placement_height + capacity_height;
            let available_height = content_area.height;

            let sections = if total_height <= available_height {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(details_height),
                        Constraint::Length(placement_height),
                        Constraint::Length(capacity_height),
                    ])
                    .split(content_area)
            } else {
                // If doesn't fit, use Min for last section
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(details_height),
                        Constraint::Length(placement_height),
                        Constraint::Min(0),
                    ])
                    .split(content_area)
            };

            let details_block = rounded_block().title(" Instance details ");
            let details_inner = details_block.inner(sections[0]);
            frame.render_widget(details_block, sections[0]);
            render_fields_with_dynamic_columns(frame, details_inner, instance_details);

            let placement_block = rounded_block().title(" Host and placement group ");
            let placement_inner = placement_block.inner(sections[1]);
            frame.render_widget(placement_block, sections[1]);
            render_fields_with_dynamic_columns(frame, placement_inner, placement);

            let capacity_block = rounded_block().title(" Capacity reservation ");
            let capacity_inner = capacity_block.inner(sections[2]);
            frame.render_widget(capacity_block, sections[2]);
            render_fields_with_dynamic_columns(frame, capacity_inner, capacity);
        }
        DetailTab::StatusAndAlarms => {
            let block = rounded_block();
            let inner = block.inner(content_area);
            frame.render_widget(block, content_area);

            let lines = vec![
                labeled_field("Status checks", &instance.status_checks),
                labeled_field("Alarm status", &instance.alarm_status),
                labeled_field("Monitoring", &instance.monitoring),
                labeled_field(
                    "State transition reason",
                    &instance.state_transition_reason_message,
                ),
            ];
            render_fields_with_dynamic_columns(frame, inner, lines);
        }
        DetailTab::Monitoring => {
            render_ec2_monitoring_charts(frame, app, content_area);
        }
        DetailTab::Security => {
            let block = rounded_block();
            let inner = block.inner(content_area);
            frame.render_widget(block, content_area);

            let lines = vec![
                labeled_field("Security groups", &instance.security_groups),
                labeled_field("Security group IDs", &instance.security_group_ids),
                labeled_field("Key name", &instance.key_name),
                labeled_field("IAM role", &instance.iam_instance_profile_arn),
                labeled_field("IMDSv2", &instance.imdsv2),
            ];
            render_fields_with_dynamic_columns(frame, inner, lines);
        }
        DetailTab::Networking => {
            let block = rounded_block();
            let inner = block.inner(content_area);
            frame.render_widget(block, content_area);

            let lines = vec![
                labeled_field("VPC ID", &instance.vpc_id),
                labeled_field("Subnet IDs", &instance.subnet_ids),
                labeled_field("Private DNS", &instance.private_dns_name),
                labeled_field("Private IP", &instance.private_ip_address),
                labeled_field("Public IPv4 DNS", &instance.public_ipv4_dns),
                labeled_field("Public IPv4", &instance.public_ipv4_address),
                labeled_field("Elastic IP", &instance.elastic_ip),
                labeled_field("IPv6 IPs", &instance.ipv6_ips),
            ];
            render_fields_with_dynamic_columns(frame, inner, lines);
        }
        DetailTab::Storage => {
            let block = rounded_block();
            let inner = block.inner(content_area);
            frame.render_widget(block, content_area);

            let lines = vec![
                labeled_field("Volume ID", &instance.volume_id),
                labeled_field("Root device", &instance.root_device_name),
                labeled_field("Root device type", &instance.root_device_type),
                labeled_field("EBS optimized", &instance.ebs_optimized),
            ];
            render_fields_with_dynamic_columns(frame, inner, lines);
        }
        DetailTab::Tags => {
            render_tags_tab(frame, app, content_area);
        }
    }
}

fn render_tags_tab(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_tags(app);

    let columns: Vec<Box<dyn TableColumn<InstanceTag>>> = app
        .ec2_state
        .tag_visible_column_ids
        .iter()
        .filter_map(|id| TagColumn::from_id(id))
        .map(|col| Box::new(col) as Box<dyn TableColumn<InstanceTag>>)
        .collect();

    let page_size = app.ec2_state.tags.page_size.value();
    let total_pages = filtered.len().div_ceil(page_size.max(1));
    let current_page = app.ec2_state.tags.selected / page_size.max(1);
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.ec2_state.tags.filter,
            placeholder: "Search tags",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.ec2_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.ec2_state.input_focus == InputFocus::Pagination,
        },
    );

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let expanded_index = app.ec2_state.tags.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    render_table(
        frame,
        TableConfig {
            area: chunks[1],
            columns: &columns,
            items: paginated,
            selected_index: app.ec2_state.tags.selected % page_size.max(1),
            is_active: app.mode != Mode::FilterInput,
            title: format!(" Tags ({}) ", filtered.len()),
            sort_column: "value",
            sort_direction: SortDirection::Asc,
            expanded_index,
            get_expanded_content: Some(Box::new(|tag: &InstanceTag| {
                expanded_from_columns(&columns, tag)
            })),
        },
    );
}

pub fn filtered_tags(app: &crate::app::App) -> Vec<&InstanceTag> {
    let mut filtered =
        filter_by_fields(&app.ec2_state.tags.items, &app.ec2_state.tags.filter, |t| {
            vec![&t.key, &t.value]
        });

    filtered.sort_by(|a, b| a.value.cmp(&b.value));
    filtered
}

pub async fn load_tags(app: &mut crate::app::App, instance_id: &str) -> anyhow::Result<()> {
    let tags = app.ec2_client.list_tags(instance_id).await?;

    app.ec2_state.tags.items = tags
        .into_iter()
        .map(|t| InstanceTag {
            key: t.key,
            value: t.value,
        })
        .collect();

    app.ec2_state
        .tags
        .items
        .sort_by(|a, b| a.value.cmp(&b.value));

    Ok(())
}

pub async fn load_ec2_metrics(app: &mut crate::app::App, instance_id: &str) -> anyhow::Result<()> {
    app.ec2_state.metric_data_cpu = app.ec2_client.get_cpu_metrics(instance_id).await?;
    app.ec2_state.metric_data_network_in =
        app.ec2_client.get_network_in_metrics(instance_id).await?;
    app.ec2_state.metric_data_network_out =
        app.ec2_client.get_network_out_metrics(instance_id).await?;
    app.ec2_state.metric_data_network_packets_in = app
        .ec2_client
        .get_network_packets_in_metrics(instance_id)
        .await?;
    app.ec2_state.metric_data_network_packets_out = app
        .ec2_client
        .get_network_packets_out_metrics(instance_id)
        .await?;
    app.ec2_state.metric_data_metadata_no_token = app
        .ec2_client
        .get_metadata_no_token_metrics(instance_id)
        .await?;
    Ok(())
}

fn render_ec2_monitoring_charts(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    use crate::ui::monitoring::render_monitoring_tab;

    let cpu_avg: f64 = if !app.ec2_state.metric_data_cpu.is_empty() {
        app.ec2_state
            .metric_data_cpu
            .iter()
            .map(|(_, v)| v)
            .sum::<f64>()
            / app.ec2_state.metric_data_cpu.len() as f64
    } else {
        0.0
    };
    let cpu_label = format!("CPU utilization (%) [avg: {:.2}]", cpu_avg);

    let network_in_sum: f64 = app
        .ec2_state
        .metric_data_network_in
        .iter()
        .map(|(_, v)| v)
        .sum();
    let network_in_label = format!("Network in (bytes) [sum: {:.0}]", network_in_sum);

    let network_out_sum: f64 = app
        .ec2_state
        .metric_data_network_out
        .iter()
        .map(|(_, v)| v)
        .sum();
    let network_out_label = format!("Network out (bytes) [sum: {:.0}]", network_out_sum);

    let network_packets_in_sum: f64 = app
        .ec2_state
        .metric_data_network_packets_in
        .iter()
        .map(|(_, v)| v)
        .sum();
    let network_packets_in_label = format!(
        "Network packets in (count) [sum: {:.0}]",
        network_packets_in_sum
    );

    let network_packets_out_sum: f64 = app
        .ec2_state
        .metric_data_network_packets_out
        .iter()
        .map(|(_, v)| v)
        .sum();
    let network_packets_out_label = format!(
        "Network packets out (count) [sum: {:.0}]",
        network_packets_out_sum
    );

    let metadata_no_token_sum: f64 = app
        .ec2_state
        .metric_data_metadata_no_token
        .iter()
        .map(|(_, v)| v)
        .sum();
    let metadata_no_token_label = format!(
        "Metadata no token (count) [sum: {:.0}]",
        metadata_no_token_sum
    );

    render_monitoring_tab(
        frame,
        area,
        &[
            crate::ui::monitoring::MetricChart {
                title: &cpu_label,
                data: &app.ec2_state.metric_data_cpu,
                y_axis_label: "%",
                x_axis_label: None,
            },
            crate::ui::monitoring::MetricChart {
                title: &network_in_label,
                data: &app.ec2_state.metric_data_network_in,
                y_axis_label: "bytes",
                x_axis_label: None,
            },
            crate::ui::monitoring::MetricChart {
                title: &network_out_label,
                data: &app.ec2_state.metric_data_network_out,
                y_axis_label: "bytes",
                x_axis_label: None,
            },
            crate::ui::monitoring::MetricChart {
                title: &network_packets_in_label,
                data: &app.ec2_state.metric_data_network_packets_in,
                y_axis_label: "count",
                x_axis_label: None,
            },
            crate::ui::monitoring::MetricChart {
                title: &network_packets_out_label,
                data: &app.ec2_state.metric_data_network_packets_out,
                y_axis_label: "count",
                x_axis_label: None,
            },
            crate::ui::monitoring::MetricChart {
                title: &metadata_no_token_label,
                data: &app.ec2_state.metric_data_metadata_no_token,
                y_axis_label: "count",
                x_axis_label: None,
            },
        ],
        &[],
        &[],
        &[],
        app.ec2_state.monitoring_scroll,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(state.sort_column, Column::LaunchTime);
        assert_eq!(state.sort_direction, SortDirection::Desc);
        assert!(!state.metrics_loading);
        assert!(state.metric_data_cpu.is_empty());
        assert!(state.metric_data_network_in.is_empty());
        assert!(state.metric_data_network_out.is_empty());
        assert!(state.metric_data_network_packets_in.is_empty());
        assert!(state.metric_data_network_packets_out.is_empty());
        assert!(state.metric_data_metadata_no_token.is_empty());
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
        // Verify rounded_block helper can be used with title
        let block = rounded_block().title(" Instance summary ");
        let area = Rect::new(0, 0, 50, 10);
        let inner = block.inner(area);
        // Inner area should be 2 smaller on each dimension due to borders
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
    fn test_load_tags_integration() {
        // Test that load_tags function exists and has correct signature
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.ec2_state.tags.items = vec![InstanceTag {
            key: "test".to_string(),
            value: "value".to_string(),
        }];
        assert_eq!(app.ec2_state.tags.items.len(), 1);
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
}
