use crate::app::App;
use crate::aws::Region;
#[cfg(test)]
use crate::common::PageSize;
use crate::common::{
    filter_by_fields, format_bytes, format_duration_seconds, format_unix_timestamp,
    render_dropdown, CyclicEnum, InputFocus, SortDirection,
};
use crate::keymap::Mode::{self, FilterInput};
use crate::sqs::pipe::Column as PipeColumn;
use crate::sqs::queue::{Column as SqsColumn, Queue};
use crate::sqs::sub::{Column as SubscriptionColumn, SnsSubscription};
use crate::sqs::tag::{Column as TagColumn, QueueTag};
use crate::sqs::trigger::Column as TriggerColumn;
use crate::sqs::{EventBridgePipe, LambdaTrigger};
use crate::table::TableState;
use crate::ui::filter::{
    render_filter_bar, render_simple_filter, FilterConfig, FilterControl, SimpleFilterConfig,
};
use crate::ui::monitoring::{render_monitoring_tab, MetricChart, MonitoringState};
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use crate::ui::{
    active_border, labeled_field, render_json_highlighted, render_pagination_text, render_tabs,
};
use ratatui::widgets::*;

pub const FILTER_CONTROLS: &[InputFocus] = &[InputFocus::Filter, InputFocus::Pagination];
pub const SUBSCRIPTION_REGION: InputFocus = InputFocus::Dropdown("SubscriptionRegion");
pub const SUBSCRIPTION_FILTER_CONTROLS: &[InputFocus] = &[
    InputFocus::Filter,
    SUBSCRIPTION_REGION,
    InputFocus::Pagination,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueueDetailTab {
    QueuePolicies,
    Monitoring,
    SnsSubscriptions,
    LambdaTriggers,
    EventBridgePipes,
    DeadLetterQueue,
    Tagging,
    Encryption,
    DeadLetterQueueRedriveTasks,
}

impl QueueDetailTab {
    pub fn all() -> Vec<QueueDetailTab> {
        vec![
            QueueDetailTab::QueuePolicies,
            QueueDetailTab::Monitoring,
            QueueDetailTab::SnsSubscriptions,
            QueueDetailTab::LambdaTriggers,
            QueueDetailTab::EventBridgePipes,
            QueueDetailTab::DeadLetterQueue,
            QueueDetailTab::Tagging,
            QueueDetailTab::Encryption,
            QueueDetailTab::DeadLetterQueueRedriveTasks,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            QueueDetailTab::QueuePolicies => "Queue policies",
            QueueDetailTab::Monitoring => "Monitoring",
            QueueDetailTab::SnsSubscriptions => "SNS subscriptions",
            QueueDetailTab::LambdaTriggers => "Lambda triggers",
            QueueDetailTab::EventBridgePipes => "EventBridge Pipes",
            QueueDetailTab::Tagging => "Tagging",
            QueueDetailTab::Encryption => "Encryption",
            QueueDetailTab::DeadLetterQueueRedriveTasks => "Dead-letter queue redrive tasks",
            QueueDetailTab::DeadLetterQueue => "Dead-letter queue",
        }
    }
}

impl CyclicEnum for QueueDetailTab {
    const ALL: &'static [Self] = &[
        QueueDetailTab::QueuePolicies,
        QueueDetailTab::Monitoring,
        QueueDetailTab::SnsSubscriptions,
        QueueDetailTab::LambdaTriggers,
        QueueDetailTab::EventBridgePipes,
        QueueDetailTab::DeadLetterQueue,
        QueueDetailTab::Tagging,
        QueueDetailTab::Encryption,
        QueueDetailTab::DeadLetterQueueRedriveTasks,
    ];
}

#[derive(Debug, Clone)]
pub struct State {
    pub queues: TableState<Queue>,
    pub triggers: TableState<LambdaTrigger>,
    pub trigger_visible_column_ids: Vec<String>,
    pub trigger_column_ids: Vec<String>,
    pub pipes: TableState<EventBridgePipe>,
    pub pipe_visible_column_ids: Vec<String>,
    pub pipe_column_ids: Vec<String>,
    pub tags: TableState<QueueTag>,
    pub tag_visible_column_ids: Vec<String>,
    pub tag_column_ids: Vec<String>,
    pub subscriptions: TableState<SnsSubscription>,
    pub subscription_visible_column_ids: Vec<String>,
    pub subscription_column_ids: Vec<String>,
    pub subscription_region_filter: String,
    pub subscription_region_selected: usize,
    pub input_focus: InputFocus,
    pub current_queue: Option<String>,
    pub detail_tab: QueueDetailTab,
    pub policy_scroll: usize,
    pub policy_document: String,
    pub metric_data: Vec<(i64, f64)>, // (timestamp, value) for ApproximateAgeOfOldestMessage
    pub metric_data_delayed: Vec<(i64, f64)>, // (timestamp, value) for ApproximateNumberOfMessagesDelayed
    pub metric_data_not_visible: Vec<(i64, f64)>, // (timestamp, value) for ApproximateNumberOfMessagesNotVisible
    pub metric_data_visible: Vec<(i64, f64)>, // (timestamp, value) for ApproximateNumberOfMessagesVisible
    pub metric_data_empty_receives: Vec<(i64, f64)>, // (timestamp, value) for NumberOfEmptyReceives
    pub metric_data_messages_deleted: Vec<(i64, f64)>, // (timestamp, value) for NumberOfMessagesDeleted
    pub metric_data_messages_received: Vec<(i64, f64)>, // (timestamp, value) for NumberOfMessagesReceived
    pub metric_data_messages_sent: Vec<(i64, f64)>, // (timestamp, value) for NumberOfMessagesSent
    pub metric_data_sent_message_size: Vec<(i64, f64)>, // (timestamp, value) for SentMessageSize
    pub metrics_loading: bool,
    pub monitoring_scroll: usize,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        let trigger_column_ids: Vec<String> = TriggerColumn::ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let pipe_column_ids: Vec<String> = PipeColumn::ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let tag_column_ids: Vec<String> = TagColumn::ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let subscription_column_ids: Vec<String> = SubscriptionColumn::ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            queues: TableState::new(),
            triggers: TableState::new(),
            trigger_visible_column_ids: trigger_column_ids.clone(),
            trigger_column_ids,
            pipes: TableState::new(),
            pipe_visible_column_ids: pipe_column_ids.clone(),
            pipe_column_ids,
            tags: TableState::new(),
            tag_visible_column_ids: tag_column_ids.clone(),
            tag_column_ids,
            subscriptions: TableState::new(),
            subscription_visible_column_ids: subscription_column_ids.clone(),
            subscription_column_ids,
            subscription_region_filter: String::new(),
            subscription_region_selected: 0,
            input_focus: InputFocus::Filter,
            current_queue: None,
            detail_tab: QueueDetailTab::QueuePolicies,
            policy_scroll: 0,
            policy_document: r#"{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": "*",
      "Action": "sqs:*",
      "Resource": "*"
    }
  ]
}"#
            .to_string(),
            metric_data: Vec::new(),
            metric_data_delayed: Vec::new(),
            metric_data_not_visible: Vec::new(),
            metric_data_visible: Vec::new(),
            metric_data_empty_receives: Vec::new(),
            metric_data_messages_deleted: Vec::new(),
            metric_data_messages_received: Vec::new(),
            metric_data_messages_sent: Vec::new(),
            metric_data_sent_message_size: Vec::new(),
            metrics_loading: false,
            monitoring_scroll: 0,
        }
    }
}

impl MonitoringState for State {
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
        self.metric_data.clear();
        self.metric_data_delayed.clear();
        self.metric_data_not_visible.clear();
        self.metric_data_visible.clear();
        self.metric_data_empty_receives.clear();
        self.metric_data_messages_deleted.clear();
        self.metric_data_messages_received.clear();
        self.metric_data_messages_sent.clear();
        self.metric_data_sent_message_size.clear();
    }
}

pub fn filtered_queues<'a>(queues: &'a [Queue], filter: &str) -> Vec<&'a Queue> {
    queues
        .iter()
        .filter(|q| filter.is_empty() || q.name.to_lowercase().starts_with(&filter.to_lowercase()))
        .collect()
}

pub fn filtered_lambda_triggers(app: &App) -> Vec<&LambdaTrigger> {
    let mut filtered = filter_by_fields(
        &app.sqs_state.triggers.items,
        &app.sqs_state.triggers.filter,
        |t| vec![&t.uuid, &t.arn],
    );

    filtered.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));
    filtered
}

pub fn filtered_tags(app: &App) -> Vec<&QueueTag> {
    let mut filtered =
        filter_by_fields(&app.sqs_state.tags.items, &app.sqs_state.tags.filter, |t| {
            vec![&t.key, &t.value]
        });

    filtered.sort_by(|a, b| a.value.cmp(&b.value));
    filtered
}

pub fn filtered_subscriptions(app: &App) -> Vec<&SnsSubscription> {
    let region_filter = if app.sqs_state.subscription_region_filter.is_empty() {
        &app.region
    } else {
        &app.sqs_state.subscription_region_filter
    };

    let mut filtered: Vec<_> = filter_by_fields(
        &app.sqs_state.subscriptions.items,
        &app.sqs_state.subscriptions.filter,
        |s| vec![&s.subscription_arn, &s.topic_arn],
    )
    .into_iter()
    .filter(|s| s.subscription_arn.contains(region_filter))
    .collect();

    filtered.sort_by(|a, b| a.subscription_arn.cmp(&b.subscription_arn));
    filtered
}

pub fn filtered_eventbridge_pipes(app: &App) -> Vec<&EventBridgePipe> {
    let mut filtered = filter_by_fields(
        &app.sqs_state.pipes.items,
        &app.sqs_state.pipes.filter,
        |p| vec![&p.name, &p.target],
    );

    filtered.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));
    filtered
}

pub async fn load_sqs_queues(app: &mut App) -> anyhow::Result<()> {
    let queues = app.sqs_client.list_queues("").await?;
    app.sqs_state.queues.items = queues
        .into_iter()
        .map(|q| Queue {
            name: q.name,
            url: q.url,
            queue_type: q.queue_type,
            created_timestamp: q.created_timestamp,
            messages_available: q.messages_available,
            messages_in_flight: q.messages_in_flight,
            encryption: q.encryption,
            content_based_deduplication: q.content_based_deduplication,
            last_modified_timestamp: q.last_modified_timestamp,
            visibility_timeout: q.visibility_timeout,
            message_retention_period: q.message_retention_period,
            maximum_message_size: q.maximum_message_size,
            delivery_delay: q.delivery_delay,
            receive_message_wait_time: q.receive_message_wait_time,
            high_throughput_fifo: q.high_throughput_fifo,
            deduplication_scope: q.deduplication_scope,
            fifo_throughput_limit: q.fifo_throughput_limit,
            dead_letter_queue: q.dead_letter_queue,
            messages_delayed: q.messages_delayed,
            redrive_allow_policy: q.redrive_allow_policy,
            redrive_policy: q.redrive_policy,
            redrive_task_id: q.redrive_task_id,
            redrive_task_start_time: q.redrive_task_start_time,
            redrive_task_status: q.redrive_task_status,
            redrive_task_percent: q.redrive_task_percent,
            redrive_task_destination: q.redrive_task_destination,
        })
        .collect();
    Ok(())
}

pub async fn load_lambda_triggers(app: &mut App, queue_url: &str) -> anyhow::Result<()> {
    let queue_arn = app.sqs_client.get_queue_arn(queue_url).await?;
    let triggers = app.sqs_client.list_lambda_triggers(&queue_arn).await?;

    app.sqs_state.triggers.items = triggers
        .into_iter()
        .map(|t| LambdaTrigger {
            uuid: t.uuid,
            arn: t.arn,
            status: t.status,
            last_modified: t.last_modified,
        })
        .collect();

    // Sort by last_modified ascending (oldest first)
    app.sqs_state
        .triggers
        .items
        .sort_by(|a, b| a.last_modified.cmp(&b.last_modified));

    Ok(())
}

pub async fn load_metrics(app: &mut App, queue_name: &str) -> anyhow::Result<()> {
    let metrics = app.sqs_client.get_queue_metrics(queue_name).await?;
    app.sqs_state.metric_data = metrics;

    let delayed_metrics = app.sqs_client.get_queue_delayed_metrics(queue_name).await?;
    app.sqs_state.metric_data_delayed = delayed_metrics;

    let not_visible_metrics = app
        .sqs_client
        .get_queue_not_visible_metrics(queue_name)
        .await?;
    app.sqs_state.metric_data_not_visible = not_visible_metrics;

    let visible_metrics = app.sqs_client.get_queue_visible_metrics(queue_name).await?;
    app.sqs_state.metric_data_visible = visible_metrics;

    let empty_receives_metrics = app
        .sqs_client
        .get_queue_empty_receives_metrics(queue_name)
        .await?;
    app.sqs_state.metric_data_empty_receives = empty_receives_metrics;

    let messages_deleted_metrics = app
        .sqs_client
        .get_queue_messages_deleted_metrics(queue_name)
        .await?;
    app.sqs_state.metric_data_messages_deleted = messages_deleted_metrics;

    let messages_received_metrics = app
        .sqs_client
        .get_queue_messages_received_metrics(queue_name)
        .await?;
    app.sqs_state.metric_data_messages_received = messages_received_metrics;

    let messages_sent_metrics = app
        .sqs_client
        .get_queue_messages_sent_metrics(queue_name)
        .await?;
    app.sqs_state.metric_data_messages_sent = messages_sent_metrics;

    let sent_message_size_metrics = app
        .sqs_client
        .get_queue_sent_message_size_metrics(queue_name)
        .await?;
    app.sqs_state.metric_data_sent_message_size = sent_message_size_metrics;

    Ok(())
}

pub async fn load_pipes(app: &mut App, queue_url: &str) -> anyhow::Result<()> {
    let queue_arn = app.sqs_client.get_queue_arn(queue_url).await?;
    let pipes = app.sqs_client.list_pipes(&queue_arn).await?;

    app.sqs_state.pipes.items = pipes
        .into_iter()
        .map(|p| EventBridgePipe {
            name: p.name,
            status: p.status,
            target: p.target,
            last_modified: p.last_modified,
        })
        .collect();

    app.sqs_state
        .pipes
        .items
        .sort_by(|a, b| a.last_modified.cmp(&b.last_modified));

    Ok(())
}

pub async fn load_tags(app: &mut App, queue_url: &str) -> anyhow::Result<()> {
    let tags = app.sqs_client.list_tags(queue_url).await?;

    app.sqs_state.tags.items = tags
        .into_iter()
        .map(|t| QueueTag {
            key: t.key,
            value: t.value,
        })
        .collect();

    app.sqs_state
        .tags
        .items
        .sort_by(|a, b| a.value.cmp(&b.value));

    Ok(())
}

pub fn render_queues(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::widgets::Clear;

    frame.render_widget(Clear, area);

    if app.sqs_state.current_queue.is_some() {
        render_queue_detail(frame, app, area);
    } else {
        render_queue_list(frame, app, area);
    }
}

fn render_queue_detail(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::prelude::*;
    use ratatui::widgets::{Clear, Paragraph};

    frame.render_widget(Clear, area);

    let queue = app
        .sqs_state
        .queues
        .items
        .iter()
        .find(|q| Some(&q.url) == app.sqs_state.current_queue.as_ref());

    let queue_name = queue.map(|q| q.name.as_str()).unwrap_or("Unknown");

    let details_height = queue.map_or(3, |q| {
        let field_count = render_details_fields(q).len();
        field_count as u16 + 2 // fields + 2 borders
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),              // Queue name
            Constraint::Length(details_height), // Details (dynamic)
            Constraint::Length(1),              // Tabs
            Constraint::Min(0),                 // Tab content
        ])
        .split(area);

    // Queue name header
    let header = Paragraph::new(queue_name).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, chunks[0]);

    // Details pane
    if let Some(q) = queue {
        render_details_pane(frame, q, chunks[1]);
    }

    // Tabs
    // Tabs - generated from QueueDetailTab::all()
    let tabs: Vec<(&str, QueueDetailTab)> = QueueDetailTab::all()
        .into_iter()
        .map(|tab| (tab.name(), tab))
        .collect();

    render_tabs(frame, chunks[2], &tabs, &app.sqs_state.detail_tab);

    // Tab content
    match app.sqs_state.detail_tab {
        QueueDetailTab::QueuePolicies => {
            render_queue_policies_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::Monitoring => {
            if app.sqs_state.metrics_loading {
                let loading_block = Block::default()
                    .title(" Monitoring ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                let loading_text = Paragraph::new("Loading metrics...")
                    .block(loading_block)
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(loading_text, chunks[3]);
            } else {
                let age_max: f64 = app
                    .sqs_state
                    .metric_data
                    .iter()
                    .map(|(_, v)| v)
                    .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let age_label = format!(
                    "Age [max: {}]",
                    if age_max.is_finite() {
                        format!("{:.0}s", age_max)
                    } else {
                        "--".to_string()
                    }
                );

                render_monitoring_tab(
                    frame,
                    chunks[3],
                    &[
                        MetricChart {
                            title: "Approximate age of oldest message",
                            data: &app.sqs_state.metric_data,
                            y_axis_label: "Seconds",
                            x_axis_label: Some(age_label),
                        },
                        MetricChart {
                            title: "Approximate number of messages delayed",
                            data: &app.sqs_state.metric_data_delayed,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Approximate number of messages not visible",
                            data: &app.sqs_state.metric_data_not_visible,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Approximate number of messages visible",
                            data: &app.sqs_state.metric_data_visible,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Number of empty receives",
                            data: &app.sqs_state.metric_data_empty_receives,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Number of messages deleted",
                            data: &app.sqs_state.metric_data_messages_deleted,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Number of messages received",
                            data: &app.sqs_state.metric_data_messages_received,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Number of messages sent",
                            data: &app.sqs_state.metric_data_messages_sent,
                            y_axis_label: "Count",
                            x_axis_label: None,
                        },
                        MetricChart {
                            title: "Sent message size",
                            data: &app.sqs_state.metric_data_sent_message_size,
                            y_axis_label: "Bytes",
                            x_axis_label: None,
                        },
                    ],
                    &[],
                    &[],
                    &[],
                    app.sqs_state.monitoring_scroll,
                );
            }
        }
        QueueDetailTab::SnsSubscriptions => {
            render_subscriptions_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::LambdaTriggers => {
            render_lambda_triggers_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::EventBridgePipes => {
            render_eventbridge_pipes_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::DeadLetterQueue => {
            render_dead_letter_queue_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::Tagging => {
            render_tags_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::Encryption => {
            render_encryption_tab(frame, app, chunks[3]);
        }
        QueueDetailTab::DeadLetterQueueRedriveTasks => {
            render_dlq_redrive_tasks_tab(frame, app, chunks[3]);
        }
    }
}

fn render_details_fields(queue: &Queue) -> Vec<ratatui::text::Line<'static>> {
    let max_msg_size = queue
        .maximum_message_size
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .map(format_bytes)
        .unwrap_or_else(|| queue.maximum_message_size.clone());

    let retention_period = queue
        .message_retention_period
        .parse::<i32>()
        .ok()
        .map(format_duration_seconds)
        .unwrap_or_else(|| queue.message_retention_period.clone());

    let visibility_timeout = queue
        .visibility_timeout
        .parse::<i32>()
        .ok()
        .map(format_duration_seconds)
        .unwrap_or_else(|| queue.visibility_timeout.clone());

    let delivery_delay = queue
        .delivery_delay
        .parse::<i32>()
        .ok()
        .map(format_duration_seconds)
        .unwrap_or_else(|| queue.delivery_delay.clone());

    let receive_wait_time = queue
        .receive_message_wait_time
        .parse::<i32>()
        .ok()
        .map(format_duration_seconds)
        .unwrap_or_else(|| queue.receive_message_wait_time.clone());

    vec![
        labeled_field("Name", &queue.name),
        labeled_field("Type", &queue.queue_type),
        labeled_field(
            "ARN",
            format!(
                "arn:aws:sqs:{}:{}:{}",
                extract_region(&queue.url),
                extract_account_id(&queue.url),
                queue.name
            ),
        ),
        labeled_field("Encryption", &queue.encryption),
        labeled_field("URL", &queue.url),
        labeled_field("Dead-letter queue", &queue.dead_letter_queue),
        labeled_field("Created", format_unix_timestamp(&queue.created_timestamp)),
        labeled_field("Maximum message size", max_msg_size),
        labeled_field(
            "Last updated",
            format_unix_timestamp(&queue.last_modified_timestamp),
        ),
        labeled_field("Message retention period", retention_period),
        labeled_field("Default visibility timeout", visibility_timeout),
        labeled_field("Messages available", &queue.messages_available),
        labeled_field("Delivery delay", delivery_delay),
        labeled_field(
            "Messages in flight (not available to other consumers)",
            &queue.messages_in_flight,
        ),
        labeled_field("Receive message wait time", receive_wait_time),
        labeled_field("Messages delayed", &queue.messages_delayed),
        labeled_field(
            "Content-based deduplication",
            &queue.content_based_deduplication,
        ),
        labeled_field("High throughput FIFO", &queue.high_throughput_fifo),
        labeled_field("Deduplication scope", &queue.deduplication_scope),
        labeled_field("FIFO throughput limit", &queue.fifo_throughput_limit),
        labeled_field("Redrive allow policy", &queue.redrive_allow_policy),
    ]
}

fn render_details_pane(frame: &mut ratatui::Frame, queue: &Queue, area: ratatui::prelude::Rect) {
    use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(active_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = render_details_fields(queue);
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_queue_policies_tab(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::prelude::{Constraint, Direction, Layout};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)])
        .split(area);

    // Access policy JSON using common JSON renderer
    render_json_highlighted(
        frame,
        chunks[0],
        &app.sqs_state.policy_document,
        app.sqs_state.policy_scroll,
        " Access policy ",
        true,
    );
}

fn render_lambda_triggers_tab(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::prelude::*;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_lambda_triggers(app);

    let columns: Vec<Box<dyn Column<LambdaTrigger>>> = app
        .sqs_state
        .trigger_visible_column_ids
        .iter()
        .filter_map(|id| TriggerColumn::from_id(id))
        .map(|col| Box::new(col) as Box<dyn Column<LambdaTrigger>>)
        .collect();

    // Pagination
    let page_size = app.sqs_state.triggers.page_size.value();
    let total_pages = filtered.len().div_ceil(page_size.max(1));
    let current_page = app.sqs_state.triggers.selected / page_size.max(1);
    let pagination = render_pagination_text(current_page, total_pages);

    // Filter at top
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.sqs_state.triggers.filter,
            placeholder: "Search triggers",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.sqs_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.sqs_state.input_focus == InputFocus::Pagination,
        },
    );

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let expanded_index = app.sqs_state.triggers.expanded_item.and_then(|idx| {
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
            selected_index: app.sqs_state.triggers.selected % page_size.max(1),
            is_active: app.mode != Mode::FilterInput,
            title: format!(" Lambda triggers ({}) ", filtered.len()),
            sort_column: "last_modified",
            sort_direction: SortDirection::Asc,
            expanded_index,
            get_expanded_content: Some(Box::new(|trigger: &LambdaTrigger| {
                expanded_from_columns(&columns, trigger)
            })),
        },
    );
}

pub fn extract_region(url: &str) -> &str {
    url.split("sqs.")
        .nth(1)
        .and_then(|s| s.split('.').next())
        .unwrap_or("unknown")
}

pub fn extract_account_id(url: &str) -> &str {
    url.split('/').nth(3).unwrap_or("unknown")
}

fn render_eventbridge_pipes_tab(
    frame: &mut ratatui::Frame,
    app: &App,
    area: ratatui::prelude::Rect,
) {
    use ratatui::prelude::*;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_eventbridge_pipes(app);

    let columns: Vec<Box<dyn Column<EventBridgePipe>>> = app
        .sqs_state
        .pipe_visible_column_ids
        .iter()
        .filter_map(|id| PipeColumn::from_id(id))
        .map(|col| Box::new(col) as Box<dyn Column<EventBridgePipe>>)
        .collect();

    let page_size = app.sqs_state.pipes.page_size.value();
    let total_pages = filtered.len().div_ceil(page_size.max(1));
    let current_page = app.sqs_state.pipes.selected / page_size.max(1);
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.sqs_state.pipes.filter,
            placeholder: "Search pipes",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.sqs_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.sqs_state.input_focus == InputFocus::Pagination,
        },
    );

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let expanded_index = app.sqs_state.pipes.expanded_item.and_then(|idx| {
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
            selected_index: app.sqs_state.pipes.selected % page_size.max(1),
            is_active: app.mode != Mode::FilterInput,
            title: format!(" EventBridge Pipes ({}) ", filtered.len()),
            sort_column: "last_modified",
            sort_direction: SortDirection::Asc,
            expanded_index,
            get_expanded_content: Some(Box::new(|pipe: &EventBridgePipe| {
                expanded_from_columns(&columns, pipe)
            })),
        },
    );
}

fn render_tags_tab(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::prelude::*;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_tags(app);

    let columns: Vec<Box<dyn Column<QueueTag>>> = app
        .sqs_state
        .tag_visible_column_ids
        .iter()
        .filter_map(|id| TagColumn::from_id(id))
        .map(|col| Box::new(col) as Box<dyn Column<QueueTag>>)
        .collect();

    let page_size = app.sqs_state.tags.page_size.value();
    let total_pages = filtered.len().div_ceil(page_size.max(1));
    let current_page = app.sqs_state.tags.selected / page_size.max(1);
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.sqs_state.tags.filter,
            placeholder: "Search tags",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.sqs_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.sqs_state.input_focus == InputFocus::Pagination,
        },
    );

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let expanded_index = app.sqs_state.tags.expanded_item.and_then(|idx| {
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
            selected_index: app.sqs_state.tags.selected % page_size.max(1),
            is_active: app.mode != Mode::FilterInput,
            title: format!(" Tagging ({}) ", filtered.len()),
            sort_column: "value",
            sort_direction: SortDirection::Asc,
            expanded_index,
            get_expanded_content: Some(Box::new(|tag: &QueueTag| {
                expanded_from_columns(&columns, tag)
            })),
        },
    );
}

fn render_subscriptions_tab(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::prelude::*;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_subscriptions(app);

    let columns: Vec<Box<dyn Column<SnsSubscription>>> = app
        .sqs_state
        .subscription_visible_column_ids
        .iter()
        .filter_map(|id| SubscriptionColumn::from_id(id))
        .map(|col| Box::new(col) as Box<dyn Column<SnsSubscription>>)
        .collect();

    let page_size = app.sqs_state.subscriptions.page_size.value();
    let total_pages = filtered.len().div_ceil(page_size.max(1));
    let current_page = app.sqs_state.subscriptions.selected / page_size.max(1);
    let pagination = render_pagination_text(current_page, total_pages);

    // Render filter with region dropdown
    render_subscription_filter(frame, app, chunks[0], &pagination);

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let expanded_index = app.sqs_state.subscriptions.expanded_item.and_then(|idx| {
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
            selected_index: app.sqs_state.subscriptions.selected % page_size.max(1),
            is_active: app.mode != Mode::FilterInput,
            title: format!(" SNS subscriptions ({}) ", filtered.len()),
            sort_column: "subscription_arn",
            sort_direction: SortDirection::Asc,
            expanded_index,
            get_expanded_content: Some(Box::new(|sub: &SnsSubscription| {
                expanded_from_columns(&columns, sub)
            })),
        },
    );

    // Render region dropdown if focused (after table so it appears on top)
    if app.mode == FilterInput && app.sqs_state.input_focus == SUBSCRIPTION_REGION {
        let regions = Region::all();
        let region_codes: Vec<&str> = regions.iter().map(|r| r.code).collect();
        render_dropdown(
            frame,
            &region_codes,
            app.sqs_state.subscription_region_selected,
            chunks[0],
            pagination.len() as u16 + 3, // pagination + separator
        );
    }
}

fn render_subscription_filter(
    frame: &mut ratatui::Frame,
    app: &App,
    area: ratatui::prelude::Rect,
    pagination: &str,
) {
    let region_text = if app.sqs_state.subscription_region_filter.is_empty() {
        format!("Subscription region: {}", app.region)
    } else {
        format!(
            "Subscription region: {}",
            app.sqs_state.subscription_region_filter
        )
    };

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.sqs_state.subscriptions.filter,
            placeholder: "Search subscriptions",
            mode: app.mode,
            is_input_focused: app.sqs_state.input_focus == InputFocus::Filter,
            controls: vec![
                FilterControl {
                    text: region_text,
                    is_focused: app.sqs_state.input_focus == SUBSCRIPTION_REGION,
                },
                FilterControl {
                    text: pagination.to_string(),
                    is_focused: app.sqs_state.input_focus == InputFocus::Pagination,
                },
            ],
            area,
        },
    );
}

fn render_dead_letter_queue_tab(
    frame: &mut ratatui::Frame,
    app: &App,
    area: ratatui::prelude::Rect,
) {
    use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

    let queue = app
        .sqs_state
        .queues
        .items
        .iter()
        .find(|q| Some(&q.url) == app.sqs_state.current_queue.as_ref());

    let block = Block::default()
        .title(" Dead-letter queue ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(active_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(q) = queue {
        if !q.redrive_policy.is_empty() {
            // Parse RedrivePolicy JSON
            if let Ok(policy) = serde_json::from_str::<serde_json::Value>(&q.redrive_policy) {
                let dlq_arn = policy
                    .get("deadLetterTargetArn")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let max_receives = policy
                    .get("maxReceiveCount")
                    .and_then(|v| v.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".to_string());

                let lines = vec![
                    labeled_field("Queue", dlq_arn),
                    labeled_field("Maximum receives", &max_receives),
                ];

                let paragraph = Paragraph::new(lines);
                frame.render_widget(paragraph, inner);
            } else {
                let paragraph = Paragraph::new("No dead-letter queue configured");
                frame.render_widget(paragraph, inner);
            }
        } else {
            let paragraph = Paragraph::new("No dead-letter queue configured");
            frame.render_widget(paragraph, inner);
        }
    }
}

fn render_encryption_tab(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

    let queue = app
        .sqs_state
        .queues
        .items
        .iter()
        .find(|q| Some(&q.url) == app.sqs_state.current_queue.as_ref());

    let block = Block::default()
        .title(" Encryption ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(active_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(q) = queue {
        let encryption_text = if q.encryption.is_empty() || q.encryption == "-" {
            "Server-side encryption is not enabled".to_string()
        } else {
            format!("Server-side encryption is managed by {}", q.encryption)
        };

        let paragraph = Paragraph::new(encryption_text);
        frame.render_widget(paragraph, inner);
    }
}

fn render_dlq_redrive_tasks_tab(
    frame: &mut ratatui::Frame,
    app: &App,
    area: ratatui::prelude::Rect,
) {
    use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

    let queue = app
        .sqs_state
        .queues
        .items
        .iter()
        .find(|q| Some(&q.url) == app.sqs_state.current_queue.as_ref());

    let block = Block::default()
        .title(" Dead-letter queue redrive status ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(active_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(q) = queue {
        let lines = vec![
            labeled_field("Name", &q.redrive_task_id),
            labeled_field("Date started", &q.redrive_task_start_time),
            labeled_field("Percent processed", &q.redrive_task_percent),
            labeled_field("Status", &q.redrive_task_status),
            labeled_field("Redrive destination", &q.redrive_task_destination),
        ];

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

fn render_queue_list(frame: &mut ratatui::Frame, app: &App, area: ratatui::prelude::Rect) {
    use ratatui::prelude::*;
    use ratatui::widgets::Clear;
    use SortDirection;

    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ])
        .split(area);

    let filtered_count =
        filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter).len();
    let page_size = app.sqs_state.queues.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.sqs_state.queues.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.sqs_state.queues.filter,
            placeholder: "Search by queue name prefix",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.sqs_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.sqs_state.input_focus == InputFocus::Pagination,
        },
    );

    let filtered: Vec<_> =
        filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter);

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let title = format!(" Queues ({}) ", filtered.len());

    let columns: Vec<Box<dyn Column<Queue>>> = app
        .sqs_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            SqsColumn::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<Queue>>)
        })
        .collect();

    let expanded_index = app.sqs_state.queues.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: paginated,
        selected_index: app.sqs_state.queues.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Name",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|queue: &Queue| {
            expanded_from_columns(&columns, queue)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqs_state_initialization() {
        let state = State::new();
        assert_eq!(state.queues.items.len(), 0);
        assert_eq!(state.queues.selected, 0);
        assert_eq!(state.queues.filter, "");
        assert_eq!(state.queues.page_size.value(), 50);
        assert_eq!(state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_filtered_queues_empty_filter() {
        let queues = vec![
            Queue {
                name: "queue1".to_string(),
                url: String::new(),
                queue_type: "Standard".to_string(),
                created_timestamp: String::new(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: String::new(),
                visibility_timeout: String::new(),
                message_retention_period: String::new(),
                maximum_message_size: String::new(),
                delivery_delay: String::new(),
                receive_message_wait_time: String::new(),
                high_throughput_fifo: "N/A".to_string(),
                deduplication_scope: "N/A".to_string(),
                fifo_throughput_limit: "N/A".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            },
            Queue {
                name: "queue2".to_string(),
                url: String::new(),
                queue_type: "Standard".to_string(),
                created_timestamp: String::new(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: String::new(),
                visibility_timeout: String::new(),
                message_retention_period: String::new(),
                maximum_message_size: String::new(),
                delivery_delay: String::new(),
                receive_message_wait_time: String::new(),
                high_throughput_fifo: "N/A".to_string(),
                deduplication_scope: "N/A".to_string(),
                fifo_throughput_limit: "N/A".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            },
        ];

        let filtered = filtered_queues(&queues, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filtered_queues_with_prefix() {
        let queues = vec![
            Queue {
                name: "prod-orders".to_string(),
                url: String::new(),
                queue_type: "Standard".to_string(),
                created_timestamp: String::new(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: String::new(),
                visibility_timeout: String::new(),
                message_retention_period: String::new(),
                maximum_message_size: String::new(),
                delivery_delay: String::new(),
                receive_message_wait_time: String::new(),
                high_throughput_fifo: "N/A".to_string(),
                deduplication_scope: "N/A".to_string(),
                fifo_throughput_limit: "N/A".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            },
            Queue {
                name: "dev-orders".to_string(),
                url: String::new(),
                queue_type: "Standard".to_string(),
                created_timestamp: String::new(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: String::new(),
                visibility_timeout: String::new(),
                message_retention_period: String::new(),
                maximum_message_size: String::new(),
                delivery_delay: String::new(),
                receive_message_wait_time: String::new(),
                high_throughput_fifo: "N/A".to_string(),
                deduplication_scope: "N/A".to_string(),
                fifo_throughput_limit: "N/A".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            },
        ];

        let filtered = filtered_queues(&queues, "prod");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "prod-orders");
    }

    #[test]
    fn test_filtered_queues_case_insensitive() {
        let queues = vec![Queue {
            name: "MyQueue".to_string(),
            url: String::new(),
            queue_type: "Standard".to_string(),
            created_timestamp: String::new(),
            messages_available: "0".to_string(),
            messages_in_flight: "0".to_string(),
            encryption: "Disabled".to_string(),
            content_based_deduplication: "Disabled".to_string(),
            last_modified_timestamp: String::new(),
            visibility_timeout: String::new(),
            message_retention_period: String::new(),
            maximum_message_size: String::new(),
            delivery_delay: String::new(),
            receive_message_wait_time: String::new(),
            high_throughput_fifo: "N/A".to_string(),
            deduplication_scope: "N/A".to_string(),
            fifo_throughput_limit: "N/A".to_string(),
            dead_letter_queue: "-".to_string(),
            messages_delayed: "0".to_string(),
            redrive_allow_policy: "-".to_string(),
            redrive_policy: "".to_string(),
            redrive_task_id: "-".to_string(),
            redrive_task_start_time: "-".to_string(),
            redrive_task_status: "-".to_string(),
            redrive_task_percent: "-".to_string(),
            redrive_task_destination: "-".to_string(),
        }];

        let filtered = filtered_queues(&queues, "my");
        assert_eq!(filtered.len(), 1);

        let filtered = filtered_queues(&queues, "MY");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_pagination_page_size() {
        let state = State::new();
        assert_eq!(state.queues.page_size.value(), 50);
    }

    #[test]
    fn test_state_initialization_with_policy() {
        let state = State::new();
        assert_eq!(state.policy_scroll, 0);
        assert_eq!(state.current_queue, None);
        assert!(state.policy_document.contains("Version"));
        assert!(state.policy_document.contains("2012-10-17"));
    }

    #[test]
    fn test_extract_region() {
        let url = "https://sqs.us-east-1.amazonaws.com/123456789012/MyQueue";
        assert_eq!(extract_region(url), "us-east-1");

        let url2 = "https://sqs.eu-west-2.amazonaws.com/987654321098/TestQueue";
        assert_eq!(extract_region(url2), "eu-west-2");
    }

    #[test]
    fn test_extract_account_id() {
        let url = "https://sqs.us-east-1.amazonaws.com/123456789012/MyQueue";
        assert_eq!(extract_account_id(url), "123456789012");

        let url2 = "https://sqs.eu-west-2.amazonaws.com/987654321098/TestQueue";
        assert_eq!(extract_account_id(url2), "987654321098");
    }

    #[test]
    fn test_timestamp_column_width() {
        use SqsColumn as Column;
        // Timestamps are 27 characters: "YYYY-MM-DD HH:MM:SS (UTC)"
        assert!(Column::Created.width() >= 27);
        assert!(Column::LastUpdated.width() >= 27);
    }

    #[test]
    fn test_message_retention_period_formatting() {
        // Test that 345600 seconds formats to days
        let seconds = 345600;
        let formatted = format_duration_seconds(seconds);
        // 345600 seconds = 4 days
        assert_eq!(formatted, "4d");
    }

    #[test]
    fn test_queue_detail_tab_navigation() {
        let tab = QueueDetailTab::QueuePolicies;
        assert_eq!(tab.next(), QueueDetailTab::Monitoring);
        assert_eq!(tab.prev(), QueueDetailTab::DeadLetterQueueRedriveTasks);

        let tab = QueueDetailTab::Monitoring;
        assert_eq!(tab.next(), QueueDetailTab::SnsSubscriptions);
        assert_eq!(tab.prev(), QueueDetailTab::QueuePolicies);

        let tab = QueueDetailTab::SnsSubscriptions;
        assert_eq!(tab.next(), QueueDetailTab::LambdaTriggers);
        assert_eq!(tab.prev(), QueueDetailTab::Monitoring);

        let tab = QueueDetailTab::LambdaTriggers;
        assert_eq!(tab.next(), QueueDetailTab::EventBridgePipes);
        assert_eq!(tab.prev(), QueueDetailTab::SnsSubscriptions);

        let tab = QueueDetailTab::EventBridgePipes;
        assert_eq!(tab.next(), QueueDetailTab::DeadLetterQueue);
        assert_eq!(tab.prev(), QueueDetailTab::LambdaTriggers);

        let tab = QueueDetailTab::DeadLetterQueue;
        assert_eq!(tab.next(), QueueDetailTab::Tagging);
        assert_eq!(tab.prev(), QueueDetailTab::EventBridgePipes);

        let tab = QueueDetailTab::Tagging;
        assert_eq!(tab.next(), QueueDetailTab::Encryption);
        assert_eq!(tab.prev(), QueueDetailTab::DeadLetterQueue);

        let tab = QueueDetailTab::Encryption;
        assert_eq!(tab.next(), QueueDetailTab::DeadLetterQueueRedriveTasks);
        assert_eq!(tab.prev(), QueueDetailTab::Tagging);

        let tab = QueueDetailTab::DeadLetterQueueRedriveTasks;
        assert_eq!(tab.next(), QueueDetailTab::QueuePolicies);
        assert_eq!(tab.prev(), QueueDetailTab::Encryption);

        let tab = QueueDetailTab::QueuePolicies;
        assert_eq!(tab.prev(), QueueDetailTab::DeadLetterQueueRedriveTasks);
    }

    #[test]
    fn test_queue_detail_tab_all() {
        let tabs = QueueDetailTab::all();
        assert_eq!(tabs.len(), 9);
        assert_eq!(tabs[0], QueueDetailTab::QueuePolicies);
        assert_eq!(tabs[1], QueueDetailTab::Monitoring);
        assert_eq!(tabs[2], QueueDetailTab::SnsSubscriptions);
        assert_eq!(tabs[3], QueueDetailTab::LambdaTriggers);
        assert_eq!(tabs[4], QueueDetailTab::EventBridgePipes);
        assert_eq!(tabs[5], QueueDetailTab::DeadLetterQueue);
        assert_eq!(tabs[6], QueueDetailTab::Tagging);
        assert_eq!(tabs[7], QueueDetailTab::Encryption);
        assert_eq!(tabs[8], QueueDetailTab::DeadLetterQueueRedriveTasks);
    }

    #[test]
    fn test_queue_detail_tab_names() {
        assert_eq!(QueueDetailTab::QueuePolicies.name(), "Queue policies");
        assert_eq!(QueueDetailTab::SnsSubscriptions.name(), "SNS subscriptions");
        assert_eq!(QueueDetailTab::LambdaTriggers.name(), "Lambda triggers");
        assert_eq!(QueueDetailTab::EventBridgePipes.name(), "EventBridge Pipes");
        assert_eq!(QueueDetailTab::Tagging.name(), "Tagging");
        assert_eq!(QueueDetailTab::DeadLetterQueue.name(), "Dead-letter queue");
    }

    #[test]
    fn test_trigger_column_all() {
        assert_eq!(TriggerColumn::all().len(), 4);
    }

    #[test]
    fn test_trigger_column_ids() {
        let ids = TriggerColumn::ids();
        assert_eq!(ids.len(), 4);
        assert!(ids.contains(&"column.sqs.trigger.uuid"));
        assert!(ids.contains(&"column.sqs.trigger.arn"));
        assert!(ids.contains(&"column.sqs.trigger.status"));
        assert!(ids.contains(&"column.sqs.trigger.last_modified"));
    }

    #[test]
    fn test_trigger_column_from_id() {
        assert_eq!(
            TriggerColumn::from_id("column.sqs.trigger.uuid"),
            Some(TriggerColumn::Uuid)
        );
        assert_eq!(
            TriggerColumn::from_id("column.sqs.trigger.arn"),
            Some(TriggerColumn::Arn)
        );
        assert_eq!(
            TriggerColumn::from_id("column.sqs.trigger.status"),
            Some(TriggerColumn::Status)
        );
        assert_eq!(
            TriggerColumn::from_id("column.sqs.trigger.last_modified"),
            Some(TriggerColumn::LastModified)
        );
        assert_eq!(TriggerColumn::from_id("invalid"), None);
    }

    #[test]
    fn test_trigger_status_rendering() {
        use Column;

        let trigger = LambdaTrigger {
            uuid: "test-uuid".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            status: "Enabled".to_string(),
            last_modified: "1609459200".to_string(),
        };

        let (text, style) = TriggerColumn::Status.render(&trigger);
        assert_eq!(text, "✅ Enabled");
        assert_eq!(style.fg, Some(ratatui::style::Color::Green));
    }

    #[test]
    fn test_trigger_timestamp_rendering() {
        use Column;

        let trigger = LambdaTrigger {
            uuid: "test-uuid".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            status: "Enabled".to_string(),
            last_modified: "1609459200".to_string(),
        };

        let (text, _) = TriggerColumn::LastModified.render(&trigger);
        assert!(text.contains("2021-01-01"));
        assert!(text.contains("(UTC)"));
    }

    #[test]
    fn test_state_initializes_trigger_columns() {
        let state = State::new();
        assert_eq!(state.trigger_column_ids.len(), 4);
        assert_eq!(state.trigger_visible_column_ids.len(), 4);
        assert_eq!(state.trigger_column_ids, state.trigger_visible_column_ids);
    }

    #[test]
    fn test_trigger_state_has_filter() {
        let mut state = State::new();
        state.detail_tab = QueueDetailTab::LambdaTriggers;
        state.triggers.filter = "test-filter".to_string();

        // Verify filter is set
        assert_eq!(state.triggers.filter, "test-filter");
        assert_eq!(state.detail_tab, QueueDetailTab::LambdaTriggers);
    }

    #[test]
    fn test_trigger_filtering() {
        let triggers = [
            LambdaTrigger {
                uuid: "uuid-123".to_string(),
                arn: "arn:aws:lambda:us-east-1:123:function:test1".to_string(),
                status: "Enabled".to_string(),
                last_modified: "1609459200".to_string(),
            },
            LambdaTrigger {
                uuid: "uuid-456".to_string(),
                arn: "arn:aws:lambda:us-east-1:123:function:test2".to_string(),
                status: "Enabled".to_string(),
                last_modified: "1609459200".to_string(),
            },
        ];

        // Filter by UUID
        let filtered: Vec<_> = triggers.iter().filter(|t| t.uuid.contains("123")).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].uuid, "uuid-123");

        // Filter by ARN
        let filtered: Vec<_> = triggers
            .iter()
            .filter(|t| t.arn.contains("test2"))
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].arn,
            "arn:aws:lambda:us-east-1:123:function:test2"
        );
    }

    #[test]
    fn test_trigger_pagination() {
        let mut state = State::new();
        state.triggers.items = (0..10)
            .map(|i| LambdaTrigger {
                uuid: format!("uuid-{}", i),
                arn: format!("arn:aws:lambda:us-east-1:123:function:test{}", i),
                status: "Enabled".to_string(),
                last_modified: "1609459200".to_string(),
            })
            .collect();

        assert_eq!(state.triggers.items.len(), 10);
        assert_eq!(state.triggers.page_size.value(), 50); // Default page size
    }

    #[test]
    fn test_trigger_column_visibility() {
        let mut state = State::new();

        // All columns visible by default
        assert_eq!(state.trigger_visible_column_ids.len(), 4);

        // Remove a column
        state.trigger_visible_column_ids.remove(0);
        assert_eq!(state.trigger_visible_column_ids.len(), 3);

        // Add it back
        state
            .trigger_visible_column_ids
            .push(state.trigger_column_ids[0].clone());
        assert_eq!(state.trigger_visible_column_ids.len(), 4);
    }

    #[test]
    fn test_trigger_page_size_options() {
        let mut state = State::new();

        // Default is 50
        assert_eq!(state.triggers.page_size, PageSize::Fifty);

        // Can change to other sizes
        state.triggers.page_size = PageSize::Ten;
        assert_eq!(state.triggers.page_size.value(), 10);

        state.triggers.page_size = PageSize::TwentyFive;
        assert_eq!(state.triggers.page_size.value(), 25);

        state.triggers.page_size = PageSize::OneHundred;
        assert_eq!(state.triggers.page_size.value(), 100);
    }

    #[test]
    fn test_trigger_loading_state() {
        let mut state = State::new();

        // Initially not loading
        assert!(!state.triggers.loading);

        // Can set to loading
        state.triggers.loading = true;
        assert!(state.triggers.loading);

        // Can clear loading
        state.triggers.loading = false;
        assert!(!state.triggers.loading);
    }

    #[test]
    fn test_trigger_sort_by_last_modified() {
        let mut triggers = [
            LambdaTrigger {
                uuid: "uuid-2".to_string(),
                arn: "arn2".to_string(),
                status: "Enabled".to_string(),
                last_modified: "1609459300".to_string(), // Later
            },
            LambdaTrigger {
                uuid: "uuid-1".to_string(),
                arn: "arn1".to_string(),
                status: "Enabled".to_string(),
                last_modified: "1609459200".to_string(), // Earlier
            },
        ];

        // Sort ascending (oldest first)
        triggers.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));

        assert_eq!(triggers[0].uuid, "uuid-1");
        assert_eq!(triggers[1].uuid, "uuid-2");
    }

    #[test]
    fn test_trigger_pagination_calculation() {
        let mut state = State::new();

        // Add 25 triggers
        state.triggers.items = (0..25)
            .map(|i| LambdaTrigger {
                uuid: format!("uuid-{}", i),
                arn: format!("arn{}", i),
                status: "Enabled".to_string(),
                last_modified: "1609459200".to_string(),
            })
            .collect();

        // With page size 10, should have 3 pages
        state.triggers.page_size = PageSize::Ten;
        let page_size = state.triggers.page_size.value();
        let total_pages = state.triggers.items.len().div_ceil(page_size);
        assert_eq!(total_pages, 3);

        // Page 0: items 0-9
        let current_page = 0;
        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(state.triggers.items.len());
        assert_eq!(start_idx, 0);
        assert_eq!(end_idx, 10);

        // Page 2: items 20-24
        let current_page = 2;
        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(state.triggers.items.len());
        assert_eq!(start_idx, 20);
        assert_eq!(end_idx, 25);
    }

    #[test]
    fn test_monitoring_metric_data_with_values() {
        let mut state = State::new();
        // Mock obfuscated metric data
        state.metric_data = vec![
            (1700000000, 0.0),
            (1700000060, 5.0),
            (1700000120, 10.0),
            (1700000180, 0.0),
        ];
        assert_eq!(state.metric_data.len(), 4);
        assert_eq!(state.metric_data[0], (1700000000, 0.0));
        assert_eq!(state.metric_data[1], (1700000060, 5.0));
    }

    #[test]
    fn test_monitoring_all_metrics_initialized() {
        let state = State::new();
        assert!(state.metric_data.is_empty());
        assert!(state.metric_data_delayed.is_empty());
        assert!(state.metric_data_not_visible.is_empty());
        assert!(state.metric_data_visible.is_empty());
        assert!(state.metric_data_empty_receives.is_empty());
        assert!(state.metric_data_messages_deleted.is_empty());
        assert!(state.metric_data_messages_received.is_empty());
        assert!(state.metric_data_messages_sent.is_empty());
        assert!(state.metric_data_sent_message_size.is_empty());
        assert_eq!(state.monitoring_scroll, 0);
    }

    #[test]
    fn test_monitoring_scroll_pages() {
        let mut state = State::new();
        assert_eq!(state.monitoring_scroll, 0);

        // Scroll to page 1
        state.monitoring_scroll = 1;
        assert_eq!(state.monitoring_scroll, 1);

        // Scroll to page 2
        state.monitoring_scroll = 2;
        assert_eq!(state.monitoring_scroll, 2);
    }

    #[test]
    fn test_monitoring_delayed_metrics() {
        let mut state = State::new();
        state.metric_data_delayed = vec![(1700000000, 1.0), (1700000060, 2.0)];
        assert_eq!(state.metric_data_delayed.len(), 2);
        assert_eq!(state.metric_data_delayed[0].1, 1.0);
    }

    #[test]
    fn test_monitoring_not_visible_metrics() {
        let mut state = State::new();
        state.metric_data_not_visible = vec![(1700000000, 3.0), (1700000060, 4.0)];
        assert_eq!(state.metric_data_not_visible.len(), 2);
        assert_eq!(state.metric_data_not_visible[1].1, 4.0);
    }

    #[test]
    fn test_monitoring_visible_metrics() {
        let mut state = State::new();
        state.metric_data_visible = vec![(1700000000, 5.0), (1700000060, 6.0)];
        assert_eq!(state.metric_data_visible.len(), 2);
        assert_eq!(state.metric_data_visible[0].1, 5.0);
    }

    #[test]
    fn test_monitoring_empty_receives_metrics() {
        let mut state = State::new();
        state.metric_data_empty_receives = vec![(1700000000, 10.0), (1700000060, 15.0)];
        assert_eq!(state.metric_data_empty_receives.len(), 2);
        assert_eq!(state.metric_data_empty_receives[0].1, 10.0);
    }

    #[test]
    fn test_monitoring_messages_deleted_metrics() {
        let mut state = State::new();
        state.metric_data_messages_deleted = vec![(1700000000, 20.0), (1700000060, 25.0)];
        assert_eq!(state.metric_data_messages_deleted.len(), 2);
        assert_eq!(state.metric_data_messages_deleted[0].1, 20.0);
    }

    #[test]
    fn test_monitoring_messages_received_metrics() {
        let mut state = State::new();
        state.metric_data_messages_received = vec![(1700000000, 30.0), (1700000060, 35.0)];
        assert_eq!(state.metric_data_messages_received.len(), 2);
        assert_eq!(state.metric_data_messages_received[0].1, 30.0);
    }

    #[test]
    fn test_monitoring_messages_sent_metrics() {
        let mut state = State::new();
        state.metric_data_messages_sent = vec![(1700000000, 40.0), (1700000060, 45.0)];
        assert_eq!(state.metric_data_messages_sent.len(), 2);
        assert_eq!(state.metric_data_messages_sent[0].1, 40.0);
    }

    #[test]
    fn test_monitoring_sent_message_size_metrics() {
        let mut state = State::new();
        state.metric_data_sent_message_size = vec![(1700000000, 1024.0), (1700000060, 2048.0)];
        assert_eq!(state.metric_data_sent_message_size.len(), 2);
        assert_eq!(state.metric_data_sent_message_size[0].1, 1024.0);
    }

    #[test]
    fn test_trigger_expand_collapse() {
        let mut state = State::new();

        // Initially no item expanded
        assert_eq!(state.triggers.expanded_item, None);

        // Expand item 0
        state.triggers.expanded_item = Some(0);
        assert_eq!(state.triggers.expanded_item, Some(0));

        // Collapse (set to None)
        state.triggers.expanded_item = None;
        assert_eq!(state.triggers.expanded_item, None);
    }

    #[test]
    fn test_trigger_filter_visibility() {
        let mut state = State::new();

        // Filter starts empty
        assert!(state.triggers.filter.is_empty());

        // Can set filter
        state.triggers.filter = "test".to_string();
        assert_eq!(state.triggers.filter, "test");

        // Can clear filter
        state.triggers.filter.clear();
        assert!(state.triggers.filter.is_empty());
    }

    #[test]
    fn test_pipe_column_ids_have_correct_prefix() {
        for col in PipeColumn::all() {
            assert!(
                col.id().starts_with("column.sqs.pipe."),
                "PipeColumn ID '{}' should start with 'column.sqs.pipe.'",
                col.id()
            );
        }
    }

    #[test]
    fn test_tags_sorted_by_value() {
        let mut state = State::new();
        state.tags.items = vec![
            QueueTag {
                key: "env".to_string(),
                value: "prod".to_string(),
            },
            QueueTag {
                key: "team".to_string(),
                value: "backend".to_string(),
            },
            QueueTag {
                key: "app".to_string(),
                value: "api".to_string(),
            },
        ];

        let mut sorted = state.tags.items.clone();
        sorted.sort_by(|a, b| a.value.cmp(&b.value));

        assert_eq!(sorted[0].value, "api");
        assert_eq!(sorted[1].value, "backend");
        assert_eq!(sorted[2].value, "prod");
    }

    #[test]
    fn test_tags_initialization() {
        let state = State::new();
        assert_eq!(state.tags.items.len(), 0);
        assert_eq!(state.tag_column_ids.len(), 2);
        assert_eq!(state.tag_visible_column_ids.len(), 2);
    }

    #[test]
    fn test_queue_tag_structure() {
        let tag = QueueTag {
            key: "Environment".to_string(),
            value: "Production".to_string(),
        };
        assert_eq!(tag.key, "Environment");
        assert_eq!(tag.value, "Production");
    }

    #[test]
    fn test_tags_table_state() {
        let mut state = State::new();
        state.tags.items = vec![
            QueueTag {
                key: "Env".to_string(),
                value: "prod".to_string(),
            },
            QueueTag {
                key: "Team".to_string(),
                value: "backend".to_string(),
            },
        ];
        assert_eq!(state.tags.items.len(), 2);
        assert_eq!(state.tags.selected, 0);
        assert_eq!(state.tags.filter, "");
    }

    #[test]
    fn test_tags_filtering() {
        let tags = [
            QueueTag {
                key: "Environment".to_string(),
                value: "production".to_string(),
            },
            QueueTag {
                key: "Team".to_string(),
                value: "backend".to_string(),
            },
            QueueTag {
                key: "Project".to_string(),
                value: "api".to_string(),
            },
        ];

        // Test filtering by key
        let filtered: Vec<_> = tags
            .iter()
            .filter(|t| t.key.to_lowercase().contains("env"))
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key, "Environment");

        // Test filtering by value
        let filtered: Vec<_> = tags
            .iter()
            .filter(|t| t.value.to_lowercase().contains("back"))
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].value, "backend");
    }

    #[test]
    fn test_tags_column_ids() {
        let ids = TagColumn::ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "column.sqs.tag.key");
        assert_eq!(ids[1], "column.sqs.tag.value");
    }

    #[test]
    fn test_tags_column_from_id() {
        assert!(TagColumn::from_id("column.sqs.tag.key").is_some());
        assert!(TagColumn::from_id("column.sqs.tag.value").is_some());
        assert!(TagColumn::from_id("invalid").is_none());
    }

    #[test]
    fn test_subscriptions_initialization() {
        let state = State::new();
        assert_eq!(state.subscriptions.items.len(), 0);
        assert_eq!(state.subscription_column_ids.len(), 2);
        assert_eq!(state.subscription_visible_column_ids.len(), 2);
        assert_eq!(state.subscription_region_filter, "");
    }

    #[test]
    fn test_subscription_column_ids() {
        let ids = SubscriptionColumn::ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "column.sqs.subscription.subscription_arn");
        assert_eq!(ids[1], "column.sqs.subscription.topic_arn");
    }

    #[test]
    fn test_subscription_column_from_id() {
        assert!(SubscriptionColumn::from_id("column.sqs.subscription.subscription_arn").is_some());
        assert!(SubscriptionColumn::from_id("column.sqs.subscription.topic_arn").is_some());
        assert!(SubscriptionColumn::from_id("invalid").is_none());
    }

    #[test]
    fn test_subscription_region_filter_default() {
        let state = State::new();
        // Default is empty string, which means use current region
        assert_eq!(state.subscription_region_filter, "");
    }

    #[test]
    fn test_subscription_region_filter_display() {
        let mut state = State::new();

        // When empty, should show current region
        assert_eq!(state.subscription_region_filter, "");

        // When set, should show selected region
        state.subscription_region_filter = "us-west-2".to_string();
        assert_eq!(state.subscription_region_filter, "us-west-2");
    }

    #[test]
    fn test_subscription_region_selected_index() {
        let state = State::new();
        assert_eq!(state.subscription_region_selected, 0);
    }

    #[test]
    fn test_encryption_tab_in_all() {
        let tabs = QueueDetailTab::all();
        assert!(tabs.contains(&QueueDetailTab::Encryption));
    }

    #[test]
    fn test_encryption_tab_name() {
        assert_eq!(QueueDetailTab::Encryption.name(), "Encryption");
    }

    #[test]
    fn test_encryption_tab_order() {
        let tabs = QueueDetailTab::all();
        let dlq_idx = tabs
            .iter()
            .position(|t| *t == QueueDetailTab::DeadLetterQueue)
            .unwrap();
        let tagging_idx = tabs
            .iter()
            .position(|t| *t == QueueDetailTab::Tagging)
            .unwrap();
        let encryption_idx = tabs
            .iter()
            .position(|t| *t == QueueDetailTab::Encryption)
            .unwrap();

        // Encryption should be after Tagging and DeadLetterQueue should be before Tagging
        assert!(dlq_idx < tagging_idx);
        assert!(encryption_idx > tagging_idx);
    }

    #[test]
    fn test_dlq_redrive_tasks_tab_in_all() {
        let tabs = QueueDetailTab::all();
        assert!(tabs.contains(&QueueDetailTab::DeadLetterQueueRedriveTasks));
    }

    #[test]
    fn test_dlq_redrive_tasks_tab_name() {
        assert_eq!(
            QueueDetailTab::DeadLetterQueueRedriveTasks.name(),
            "Dead-letter queue redrive tasks"
        );
    }

    #[test]
    fn test_dlq_redrive_tasks_tab_order() {
        let tabs = QueueDetailTab::all();
        let encryption_idx = tabs
            .iter()
            .position(|t| *t == QueueDetailTab::Encryption)
            .unwrap();
        let redrive_idx = tabs
            .iter()
            .position(|t| *t == QueueDetailTab::DeadLetterQueueRedriveTasks)
            .unwrap();

        // DeadLetterQueueRedriveTasks should be after Encryption (last tab)
        assert!(redrive_idx > encryption_idx);
        assert_eq!(redrive_idx, tabs.len() - 1);
    }

    #[test]
    fn test_tab_strip_matches_enum_order() {
        // This test ensures the hardcoded tab strip in render_queue_detail matches QueueDetailTab::all()
        let all_tabs = QueueDetailTab::all();
        assert_eq!(all_tabs.len(), 9);

        // Verify order matches
        assert_eq!(all_tabs[0], QueueDetailTab::QueuePolicies);
        assert_eq!(all_tabs[1], QueueDetailTab::Monitoring);
        assert_eq!(all_tabs[2], QueueDetailTab::SnsSubscriptions);
        assert_eq!(all_tabs[3], QueueDetailTab::LambdaTriggers);
        assert_eq!(all_tabs[4], QueueDetailTab::EventBridgePipes);
        assert_eq!(all_tabs[5], QueueDetailTab::DeadLetterQueue);
        assert_eq!(all_tabs[6], QueueDetailTab::Tagging);
        assert_eq!(all_tabs[7], QueueDetailTab::Encryption);
        assert_eq!(all_tabs[8], QueueDetailTab::DeadLetterQueueRedriveTasks);
    }

    #[test]
    fn test_monitoring_tab_in_all() {
        let all_tabs = QueueDetailTab::all();
        assert!(all_tabs.contains(&QueueDetailTab::Monitoring));
    }

    #[test]
    fn test_monitoring_tab_name() {
        assert_eq!(QueueDetailTab::Monitoring.name(), "Monitoring");
    }

    #[test]
    fn test_monitoring_tab_order() {
        let all_tabs = QueueDetailTab::all();
        let monitoring_index = all_tabs
            .iter()
            .position(|t| *t == QueueDetailTab::Monitoring)
            .unwrap();
        assert_eq!(monitoring_index, 1); // Should be second, after QueuePolicies
    }
}
