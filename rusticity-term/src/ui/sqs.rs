use crate::common::InputFocus;
use crate::sqs::Queue;
use crate::table::TableState;

pub const FILTER_CONTROLS: &[InputFocus] = &[InputFocus::Filter, InputFocus::Pagination];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueueDetailTab {
    QueuePolicies,
}

impl QueueDetailTab {
    pub fn all() -> Vec<QueueDetailTab> {
        vec![QueueDetailTab::QueuePolicies]
    }

    pub fn name(&self) -> &str {
        match self {
            QueueDetailTab::QueuePolicies => "Queue policies",
        }
    }

    pub fn next(&self) -> Self {
        // Only one tab for now
        *self
    }

    pub fn prev(&self) -> Self {
        // Only one tab for now
        *self
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub queues: TableState<Queue>,
    pub input_focus: InputFocus,
    pub current_queue: Option<String>,
    pub detail_tab: QueueDetailTab,
    pub policy_scroll: usize,
    pub policy_document: String,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            queues: TableState::new(),
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
        }
    }
}

pub fn filtered_queues<'a>(queues: &'a [Queue], filter: &str) -> Vec<&'a Queue> {
    queues
        .iter()
        .filter(|q| filter.is_empty() || q.name.to_lowercase().starts_with(&filter.to_lowercase()))
        .collect()
}

pub async fn load_sqs_queues(app: &mut crate::App) -> anyhow::Result<()> {
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
        })
        .collect();
    Ok(())
}

pub fn render_queues(frame: &mut ratatui::Frame, app: &crate::App, area: ratatui::prelude::Rect) {
    use ratatui::widgets::Clear;

    frame.render_widget(Clear, area);

    if app.sqs_state.current_queue.is_some() {
        render_queue_detail(frame, app, area);
    } else {
        render_queue_list(frame, app, area);
    }
}

fn render_queue_detail(frame: &mut ratatui::Frame, app: &crate::App, area: ratatui::prelude::Rect) {
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Queue name
            Constraint::Length(24), // Details (22 lines + 2 borders)
            Constraint::Length(1),  // Tabs
            Constraint::Min(0),     // Tab content
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
    crate::ui::render_tabs(
        frame,
        chunks[2],
        &[("Queue policies", QueueDetailTab::QueuePolicies)],
        &app.sqs_state.detail_tab,
    );

    // Tab content
    match app.sqs_state.detail_tab {
        QueueDetailTab::QueuePolicies => {
            render_queue_policies_tab(frame, app, chunks[3]);
        }
    }
}

fn render_details_pane(frame: &mut ratatui::Frame, queue: &Queue, area: ratatui::prelude::Rect) {
    use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(crate::ui::active_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let max_msg_size = queue
        .maximum_message_size
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .map(crate::common::format_bytes)
        .unwrap_or_else(|| queue.maximum_message_size.clone());

    let retention_period = queue
        .message_retention_period
        .parse::<i32>()
        .ok()
        .map(crate::common::format_duration_seconds)
        .unwrap_or_else(|| queue.message_retention_period.clone());

    let lines = vec![
        crate::ui::labeled_field("Name", &queue.name),
        crate::ui::labeled_field("Type", &queue.queue_type),
        crate::ui::labeled_field(
            "ARN",
            format!(
                "arn:aws:sqs:{}:{}:{}",
                extract_region(&queue.url),
                extract_account_id(&queue.url),
                queue.name
            ),
        ),
        crate::ui::labeled_field("Encryption", &queue.encryption),
        crate::ui::labeled_field("URL", &queue.url),
        crate::ui::labeled_field("Dead-letter queue", "-"),
        crate::ui::labeled_field(
            "Created",
            crate::common::format_unix_timestamp(&queue.created_timestamp),
        ),
        crate::ui::labeled_field("Maximum message size", max_msg_size),
        crate::ui::labeled_field(
            "Last updated",
            crate::common::format_unix_timestamp(&queue.last_modified_timestamp),
        ),
        crate::ui::labeled_field("Message retention period", retention_period),
        crate::ui::labeled_field("Default visibility timeout", &queue.visibility_timeout),
        crate::ui::labeled_field("Messages available", &queue.messages_available),
        crate::ui::labeled_field("Delivery delay", &queue.delivery_delay),
        crate::ui::labeled_field(
            "Messages in flight (not available to other consumers)",
            &queue.messages_in_flight,
        ),
        crate::ui::labeled_field(
            "Receive message wait time",
            &queue.receive_message_wait_time,
        ),
        crate::ui::labeled_field("Messages delayed", "0"),
        crate::ui::labeled_field(
            "Content-based deduplication",
            &queue.content_based_deduplication,
        ),
        crate::ui::labeled_field("High throughput FIFO", &queue.high_throughput_fifo),
        crate::ui::labeled_field("Deduplication scope", &queue.deduplication_scope),
        crate::ui::labeled_field("FIFO throughput limit", &queue.fifo_throughput_limit),
        crate::ui::labeled_field("Redrive allow policy", "-"),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_queue_policies_tab(
    frame: &mut ratatui::Frame,
    app: &crate::App,
    area: ratatui::prelude::Rect,
) {
    use ratatui::prelude::{Color, Constraint, Direction, Layout, Style};
    use ratatui::widgets::Paragraph;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Description
    let desc =
        Paragraph::new("Define who can access your queue.").style(Style::default().fg(Color::Gray));
    frame.render_widget(desc, chunks[0]);

    // Access policy JSON using common JSON renderer
    crate::ui::render_json_highlighted(
        frame,
        chunks[1],
        &app.sqs_state.policy_document,
        app.sqs_state.policy_scroll,
        " Access policy ",
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

fn render_queue_list(frame: &mut ratatui::Frame, app: &crate::App, area: ratatui::prelude::Rect) {
    use crate::common::SortDirection;
    use crate::keymap::Mode;
    use ratatui::prelude::*;
    use ratatui::widgets::Clear;

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
    let pagination = crate::ui::render_pagination_text(current_page, total_pages);

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
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

    let columns: Vec<Box<dyn crate::ui::table::Column<Queue>>> = app
        .visible_sqs_columns
        .iter()
        .map(|col| Box::new(*col) as Box<dyn crate::ui::table::Column<Queue>>)
        .collect();

    let expanded_index = app.sqs_state.queues.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    let config = crate::ui::table::TableConfig {
        items: paginated,
        selected_index: app.sqs_state.queues.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Name",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|queue: &Queue| {
            crate::ui::table::expanded_from_columns(&columns, queue)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    crate::ui::table::render_table(frame, config);
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
        use crate::sqs::Column;
        use crate::ui::table::Column as TableColumn;
        // Timestamps are 27 characters: "YYYY-MM-DD HH:MM:SS (UTC)"
        assert!(Column::Created.width() >= 27);
        assert!(Column::LastUpdated.width() >= 27);
    }

    #[test]
    fn test_message_retention_period_formatting() {
        // Test that 345600 seconds formats to days/hours
        let seconds = 345600;
        let formatted = crate::common::format_duration_seconds(seconds);
        // 345600 seconds = 4 days = 5760 minutes
        assert!(formatted.contains("5760min") || formatted.contains("day"));
    }

    #[test]
    fn test_queue_detail_tab_navigation() {
        let tab = QueueDetailTab::QueuePolicies;
        assert_eq!(tab.next(), QueueDetailTab::QueuePolicies);
        assert_eq!(tab.prev(), QueueDetailTab::QueuePolicies);
    }

    #[test]
    fn test_queue_detail_tab_all() {
        let tabs = QueueDetailTab::all();
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0], QueueDetailTab::QueuePolicies);
    }

    #[test]
    fn test_queue_detail_tab_names() {
        assert_eq!(QueueDetailTab::QueuePolicies.name(), "Queue policies");
    }
}
