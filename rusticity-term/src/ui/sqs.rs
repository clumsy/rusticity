use crate::common::InputFocus;
use crate::sqs::Queue;
use crate::table::TableState;

pub const FILTER_CONTROLS: &[InputFocus] = &[InputFocus::Filter, InputFocus::Pagination];

#[derive(Debug, Clone)]
pub struct State {
    pub queues: TableState<Queue>,
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
            queues: TableState::new(),
            input_focus: InputFocus::Filter,
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
}
