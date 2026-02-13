use crate::app::{App, CloudTrailDetailFocus};
use crate::cloudtrail::{CloudTrailEvent, CloudTrailEventColumn};
use crate::common::{InputFocus, SortDirection};
use crate::keymap::Mode;
use crate::ui::table::{render_table, Column, TableConfig};
use crate::ui::{format_title, vertical};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::Frame;

struct CloudTrailEventTableColumn {
    column_type: CloudTrailEventColumn,
}

impl Column<CloudTrailEvent> for CloudTrailEventTableColumn {
    fn name(&self) -> &str {
        Box::leak(self.column_type.name().into_boxed_str())
    }

    fn width(&self) -> u16 {
        self.column_type.width()
    }

    fn render(&self, event: &CloudTrailEvent) -> (String, Style) {
        match self.column_type {
            CloudTrailEventColumn::EventName => (event.event_name.clone(), Style::default()),
            CloudTrailEventColumn::EventTime => (event.event_time.clone(), Style::default()),
            CloudTrailEventColumn::Username => (event.username.clone(), Style::default()),
            CloudTrailEventColumn::EventSource => (event.event_source.clone(), Style::default()),
            CloudTrailEventColumn::ResourceType => (event.resource_type.clone(), Style::default()),
            CloudTrailEventColumn::ResourceName => (event.resource_name.clone(), Style::default()),
            CloudTrailEventColumn::ReadOnly => (event.read_only.clone(), Style::default()),
            CloudTrailEventColumn::AwsRegion => (event.aws_region.clone(), Style::default()),
            CloudTrailEventColumn::EventId => (event.event_id.clone(), Style::default()),
            CloudTrailEventColumn::AccessKeyId => (event.access_key_id.clone(), Style::default()),
            CloudTrailEventColumn::SourceIpAddress => {
                (event.source_ip_address.clone(), Style::default())
            }
            CloudTrailEventColumn::ErrorCode => (event.error_code.clone(), Style::default()),
            CloudTrailEventColumn::RequestId => (event.request_id.clone(), Style::default()),
            CloudTrailEventColumn::EventType => (event.event_type.clone(), Style::default()),
        }
    }
}

pub fn render_events(frame: &mut Frame, app: &App, area: Rect) {
    if app.cloudtrail_state.current_event.is_some() {
        render_event_detail(frame, app, area);
        return;
    }

    let chunks = vertical([Constraint::Length(3), Constraint::Min(0)], area);

    // Filter
    let page_size = app.cloudtrail_state.table.page_size.value();
    let filtered_count = app.cloudtrail_state.table.items.len();
    let loaded_pages = filtered_count.div_ceil(page_size);
    let current_page = app.cloudtrail_state.table.selected / page_size;

    // Show loaded pages + 1 if more data available
    let max_page = if app.cloudtrail_state.table.next_token.is_some() {
        loaded_pages + 1
    } else {
        loaded_pages
    };

    let pagination = (0..max_page)
        .map(|i| {
            if i == current_page {
                format!("[{}]", i + 1)
            } else {
                format!("{}", i + 1)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        + if app.cloudtrail_state.table.next_token.is_some() {
            " ..."
        } else {
            ""
        };

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
            filter_text: &app.cloudtrail_state.table.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.cloudtrail_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.cloudtrail_state.input_focus == InputFocus::Pagination,
        },
    );

    let filtered_events: Vec<&CloudTrailEvent> = app.cloudtrail_state.table.items.iter().collect();

    // Apply pagination
    let page_size = app.cloudtrail_state.table.page_size.value();
    let current_page = app.cloudtrail_state.table.selected / page_size;
    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered_events.len());

    // If navigated beyond loaded items, show empty page
    let paginated: Vec<_> = if start_idx < filtered_events.len() {
        filtered_events[start_idx..end_idx].to_vec()
    } else {
        Vec::new()
    };

    let title = format_title("CloudTrail Events");

    let columns: Vec<Box<dyn Column<CloudTrailEvent>>> = app
        .cloudtrail_event_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            CloudTrailEventColumn::from_id(col_id).map(|col| {
                Box::new(CloudTrailEventTableColumn { column_type: col })
                    as Box<dyn Column<CloudTrailEvent>>
            })
        })
        .collect();

    let config = TableConfig {
        items: paginated,
        selected_index: app.cloudtrail_state.table.selected % page_size,
        expanded_index: app.cloudtrail_state.table.expanded_item.and_then(|idx| {
            if idx >= start_idx && idx < end_idx {
                Some(idx - start_idx)
            } else {
                None
            }
        }),
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[1],
        is_active: !matches!(
            app.mode,
            Mode::SpaceMenu
                | Mode::ServicePicker
                | Mode::ColumnSelector
                | Mode::ErrorModal
                | Mode::HelpModal
                | Mode::RegionPicker
                | Mode::CalendarPicker
                | Mode::TabPicker
                | Mode::FilterInput
        ),
        get_expanded_content: Some(Box::new(|event: &CloudTrailEvent| {
            crate::ui::table::expanded_from_columns(&columns, event)
        })),
    };

    render_table(frame, config);
}

fn render_event_detail(frame: &mut Frame, app: &App, area: Rect) {
    use crate::ui::{
        calculate_dynamic_height, format_title, labeled_field, render_fields_with_dynamic_columns,
        render_json_highlighted, rounded_block,
    };
    use ratatui::layout::{Direction, Layout};

    let event = app.cloudtrail_state.current_event.as_ref().unwrap();

    let fields = vec![
        labeled_field("Event time", &event.event_time),
        labeled_field("User name", &event.username),
        labeled_field("Event name", &event.event_name),
        labeled_field("Event source", &event.event_source),
        labeled_field("AWS access key", &event.access_key_id),
        labeled_field("Source IP address", &event.source_ip_address),
        labeled_field("Event ID", &event.event_id),
        labeled_field("Request ID", &event.request_id),
        labeled_field("AWS region", &event.aws_region),
        labeled_field(
            "Error code",
            if event.error_code.is_empty() {
                "-"
            } else {
                &event.error_code
            },
        ),
        labeled_field("Read-only", &event.read_only),
    ];

    let details_height = calculate_dynamic_height(&fields, area.width.saturating_sub(4)) + 2;
    let has_resources = !event.resource_type.is_empty() && !event.resource_name.is_empty();
    let resource_count = if has_resources { 1 } else { 0 };
    let visible_column_count = app.cloudtrail_resource_visible_column_ids.len();
    let resources_height = if has_resources {
        // Always allocate space for expansion: (n_rows + n_visible_cols - 1) + 1 table header + 2 borders + 1 title
        (resource_count + visible_column_count - 1 + 1 + 2 + 1) as u16
    } else {
        3 // Empty block with borders + title
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(details_height),
            Constraint::Length(resources_height),
            Constraint::Min(0),
        ])
        .split(area);

    let block = rounded_block().title(format_title("Details"));
    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);
    render_fields_with_dynamic_columns(frame, inner, fields);

    if has_resources {
        use crate::cloudtrail::{EventResource, EventResourceColumn};

        let resource = EventResource {
            resource_type: event.resource_type.clone(),
            resource_name: event.resource_name.clone(),
            timeline: "-".to_string(),
        };

        let all_columns: Vec<EventResourceColumn> = app
            .cloudtrail_resource_visible_column_ids
            .iter()
            .filter_map(EventResourceColumn::from_id)
            .collect();

        let columns: Vec<Box<dyn Column<EventResource>>> = all_columns
            .iter()
            .map(|col| Box::new(*col) as Box<dyn Column<EventResource>>)
            .collect();

        let config = TableConfig {
            items: vec![&resource],
            selected_index: 0,
            expanded_index: app.cloudtrail_state.resources_expanded_index,
            columns: &columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title: format_title(&format!("Resources referenced ({})", resource_count)),
            area: chunks[1],
            is_active: app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::Resources,
            get_expanded_content: Some(Box::new(|resource: &EventResource| {
                crate::ui::table::expanded_from_columns(&columns, resource)
            })),
        };

        render_table(frame, config);
    } else {
        let resources_block = rounded_block()
            .title(format_title(&format!(
                "Resources referenced ({})",
                resource_count
            )))
            .border_style(
                if app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::Resources {
                    crate::ui::styles::active_border()
                } else {
                    Style::default()
                },
            );
        frame.render_widget(resources_block, chunks[1]);
    }

    let is_json_active = app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::EventRecord;

    render_json_highlighted(
        frame,
        chunks[2],
        &event.cloud_trail_event_json,
        app.cloudtrail_state.event_json_scroll,
        "Event record",
        is_json_active,
    );
}

#[cfg(test)]
mod tests {
    use crate::ui::labeled_field;
    use ratatui::style::Modifier;

    #[test]
    fn test_json_scroll_bounds() {
        let mut scroll = 0usize;

        scroll = scroll.saturating_sub(10);
        assert_eq!(scroll, 0);

        scroll = 5;
        scroll = scroll.saturating_sub(10);
        assert_eq!(scroll, 0);
    }

    #[test]
    fn test_labeled_field_bolds_label() {
        let line = labeled_field("Test Label", "test value");
        assert_eq!(line.spans.len(), 2);
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(line.spans[0].content, "Test Label: ");
        assert_eq!(line.spans[1].content, "test value");
    }

    #[test]
    fn test_event_json_has_newlines() {
        use crate::cloudtrail::CloudTrailEvent;

        let event = CloudTrailEvent {
            event_time: "2024-01-01T12:00:00Z".to_string(),
            event_name: "CreateBucket".to_string(),
            username: "test-user".to_string(),
            event_source: "s3.amazonaws.com".to_string(),
            resource_type: "AWS::S3::Bucket".to_string(),
            resource_name: "test-bucket".to_string(),
            read_only: "false".to_string(),
            aws_region: "us-east-1".to_string(),
            event_id: "12345".to_string(),
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            source_ip_address: "192.0.2.1".to_string(),
            error_code: "".to_string(),
            request_id: "req-12345".to_string(),
            event_type: "AwsApiCall".to_string(),
            cloud_trail_event_json: r#"{
  "eventVersion": "1.08",
  "userIdentity": {
    "type": "IAMUser"
  }
}"#
            .to_string(),
        };

        assert!(
            event.cloud_trail_event_json.contains('\n'),
            "Event JSON should contain newlines for proper formatting"
        );
        assert!(event.cloud_trail_event_json.lines().count() > 1);
    }

    #[test]
    fn test_event_json_scroll_bounds() {
        let json_with_50_lines = (0..50)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let line_count = json_with_50_lines.lines().count();

        let mut scroll = 0usize;

        // Scroll down
        scroll = (scroll + 10).min(line_count.saturating_sub(1));
        assert_eq!(scroll, 10);

        // Scroll down again
        scroll = (scroll + 10).min(line_count.saturating_sub(1));
        assert_eq!(scroll, 20);

        // Scroll up
        scroll = scroll.saturating_sub(10);
        assert_eq!(scroll, 10);

        // Scroll up to beginning
        scroll = scroll.saturating_sub(10);
        assert_eq!(scroll, 0);

        // Can't scroll below 0
        scroll = scroll.saturating_sub(10);
        assert_eq!(scroll, 0);
    }

    #[test]
    fn test_detail_focus_cycles_with_tab_and_shift_tab() {
        use crate::app::CloudTrailDetailFocus;
        use crate::common::CyclicEnum;

        let mut focus = CloudTrailDetailFocus::Resources;

        // Tab (next) cycles to EventRecord
        focus = focus.next();
        assert_eq!(focus, CloudTrailDetailFocus::EventRecord);

        // Tab (next) cycles back to Resources
        focus = focus.next();
        assert_eq!(focus, CloudTrailDetailFocus::Resources);

        // Shift+Tab (prev) cycles to EventRecord
        focus = focus.prev();
        assert_eq!(focus, CloudTrailDetailFocus::EventRecord);

        // Shift+Tab (prev) cycles back to Resources
        focus = focus.prev();
        assert_eq!(focus, CloudTrailDetailFocus::Resources);
    }

    #[test]
    fn test_console_url_includes_event_id() {
        let event_id = "90c72977-31e0-4079-9a74-ee25e5d7aadf";
        let region = "us-east-1";

        let url = format!(
            "https://{}.console.aws.amazon.com/cloudtrailv2/home?region={}#/events/{}",
            region, region, event_id
        );

        assert!(url.contains(event_id));
        assert!(url.contains("cloudtrailv2"));
        assert_eq!(
            url,
            "https://us-east-1.console.aws.amazon.com/cloudtrailv2/home?region=us-east-1#/events/90c72977-31e0-4079-9a74-ee25e5d7aadf"
        );
    }

    #[test]
    fn test_event_resource_column_renders_all_three_columns() {
        use crate::cloudtrail::{EventResource, EventResourceColumn};
        use crate::ui::table::Column;

        let resource = EventResource {
            resource_type: "AWS::S3::Bucket".to_string(),
            resource_name: "my-bucket".to_string(),
            timeline: "-".to_string(),
        };

        // Test Resource type column
        let col1 = EventResourceColumn::ResourceType;
        assert_eq!(col1.name(), "Resource type");
        assert_eq!(col1.width(), 30);
        let (value, _) = col1.render(&resource);
        assert_eq!(value, "AWS::S3::Bucket");

        // Test Resource name column
        let col2 = EventResourceColumn::ResourceName;
        assert_eq!(col2.name(), "Resource name");
        assert_eq!(col2.width(), 50);
        let (value, _) = col2.render(&resource);
        assert_eq!(value, "my-bucket");

        // Test AWS Config resource timeline column
        let col3 = EventResourceColumn::Timeline;
        assert_eq!(col3.name(), "AWS Config resource timeline");
        assert_eq!(col3.width(), 30);
        let (value, _) = col3.render(&resource);
        assert_eq!(value, "-");
    }
}
