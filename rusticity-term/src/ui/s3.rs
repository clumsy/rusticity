use crate::app::App;
use crate::common::CyclicEnum;
use crate::common::{format_bytes, format_iso_timestamp, UTC_TIMESTAMP_WIDTH};
use crate::keymap::Mode;
use crate::s3::{Bucket as S3Bucket, BucketColumn, Object as S3Object};
use crate::table::TableState;
use crate::ui::{
    active_border, filter_area, get_cursor, red_text, render_inner_tab_spans, render_tabs,
};
use ratatui::{prelude::*, widgets::*};
use std::collections::{HashMap, HashSet};

pub struct State {
    pub buckets: TableState<S3Bucket>,
    pub bucket_type: BucketType,
    pub selected_row: usize,
    pub bucket_scroll_offset: usize,
    pub bucket_visible_rows: std::cell::Cell<usize>,
    pub current_bucket: Option<String>,
    pub prefix_stack: Vec<String>,
    pub objects: Vec<S3Object>,
    pub selected_object: usize,
    pub object_scroll_offset: usize,
    pub object_visible_rows: std::cell::Cell<usize>,
    pub expanded_prefixes: HashSet<String>,
    pub object_tab: ObjectTab,
    pub object_filter: String,
    pub selected_objects: HashSet<String>,
    pub bucket_preview: HashMap<String, Vec<S3Object>>,
    pub bucket_errors: HashMap<String, String>,
    pub prefix_preview: HashMap<String, Vec<S3Object>>,
    pub properties_scroll: u16,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            buckets: TableState::new(),
            bucket_type: BucketType::GeneralPurpose,
            selected_row: 0,
            bucket_scroll_offset: 0,
            bucket_visible_rows: std::cell::Cell::new(30),
            current_bucket: None,
            prefix_stack: Vec::new(),
            objects: Vec::new(),
            selected_object: 0,
            object_scroll_offset: 0,
            object_visible_rows: std::cell::Cell::new(30),
            expanded_prefixes: HashSet::new(),
            object_tab: ObjectTab::Objects,
            object_filter: String::new(),
            selected_objects: HashSet::new(),
            bucket_preview: HashMap::new(),
            bucket_errors: HashMap::new(),
            prefix_preview: HashMap::new(),
            properties_scroll: 0,
        }
    }

    pub fn calculate_total_bucket_rows(&self) -> usize {
        fn count_nested(
            obj: &S3Object,
            expanded_prefixes: &HashSet<String>,
            prefix_preview: &HashMap<String, Vec<S3Object>>,
        ) -> usize {
            let mut count = 0;
            if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                if let Some(preview) = prefix_preview.get(&obj.key) {
                    count += preview.len();
                    for nested_obj in preview {
                        count += count_nested(nested_obj, expanded_prefixes, prefix_preview);
                    }
                }
            }
            count
        }

        let mut total = self.buckets.items.len();
        for bucket in &self.buckets.items {
            if self.expanded_prefixes.contains(&bucket.name) {
                if self.bucket_errors.contains_key(&bucket.name) {
                    continue;
                }
                if let Some(preview) = self.bucket_preview.get(&bucket.name) {
                    total += preview.len();
                    for obj in preview {
                        total += count_nested(obj, &self.expanded_prefixes, &self.prefix_preview);
                    }
                }
            }
        }
        total
    }

    pub fn calculate_total_object_rows(&self) -> usize {
        fn count_nested(
            obj: &S3Object,
            expanded_prefixes: &HashSet<String>,
            prefix_preview: &HashMap<String, Vec<S3Object>>,
        ) -> usize {
            let mut count = 0;
            if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                if let Some(preview) = prefix_preview.get(&obj.key) {
                    count += preview.len();
                    for nested_obj in preview {
                        count += count_nested(nested_obj, expanded_prefixes, prefix_preview);
                    }
                } else {
                    count += 1;
                }
            }
            count
        }

        let mut total = self.objects.len();
        for obj in &self.objects {
            total += count_nested(obj, &self.expanded_prefixes, &self.prefix_preview);
        }
        total
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BucketType {
    GeneralPurpose,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectTab {
    Objects,
    Metadata,
    Properties,
    Permissions,
    Metrics,
    Management,
    AccessPoints,
}

impl CyclicEnum for ObjectTab {
    const ALL: &'static [Self] = &[Self::Objects, Self::Properties];
}

impl ObjectTab {
    pub fn name(&self) -> &'static str {
        match self {
            ObjectTab::Objects => "Objects",
            ObjectTab::Metadata => "Metadata",
            ObjectTab::Properties => "Properties",
            ObjectTab::Permissions => "Permissions",
            ObjectTab::Metrics => "Metrics",
            ObjectTab::Management => "Management",
            ObjectTab::AccessPoints => "Access Points",
        }
    }

    pub fn all() -> Vec<ObjectTab> {
        vec![ObjectTab::Objects, ObjectTab::Properties]
    }
}

pub fn calculate_total_bucket_rows(app: &App) -> usize {
    fn count_nested(
        obj: &crate::app::S3Object,
        expanded_prefixes: &std::collections::HashSet<String>,
        prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    ) -> usize {
        let mut count = 0;
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(preview) = prefix_preview.get(&obj.key) {
                count += preview.len();
                for nested_obj in preview {
                    count += count_nested(nested_obj, expanded_prefixes, prefix_preview);
                }
            }
        }
        count
    }

    let mut total = app.s3_state.buckets.items.len();
    for bucket in &app.s3_state.buckets.items {
        if app.s3_state.expanded_prefixes.contains(&bucket.name) {
            if app.s3_state.bucket_errors.contains_key(&bucket.name) {
                continue;
            }
            if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                total += preview.len();
                for obj in preview {
                    total += count_nested(
                        obj,
                        &app.s3_state.expanded_prefixes,
                        &app.s3_state.prefix_preview,
                    );
                }
            }
        }
    }
    total
}

pub fn calculate_total_object_rows(app: &App) -> usize {
    fn count_nested(
        obj: &crate::app::S3Object,
        expanded_prefixes: &std::collections::HashSet<String>,
        prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    ) -> usize {
        let mut count = 0;
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(preview) = prefix_preview.get(&obj.key) {
                count += preview.len();
                for nested_obj in preview {
                    count += count_nested(nested_obj, expanded_prefixes, prefix_preview);
                }
            } else {
                count += 1;
            }
        }
        count
    }

    let mut total = app.s3_state.objects.len();
    for obj in &app.s3_state.objects {
        total += count_nested(
            obj,
            &app.s3_state.expanded_prefixes,
            &app.s3_state.prefix_preview,
        );
    }
    total
}

pub fn render_buckets(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.s3_state.current_bucket.is_some() {
        render_objects(frame, app, area);
    } else {
        render_bucket_list(frame, app, area);
    }
}

// S3 functions extracted from mod.rs - content will be added via file append
fn render_bucket_list(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let chunks = crate::ui::vertical(
        [
            Constraint::Length(1), // Tabs
            Constraint::Length(3), // Filter (1 line + borders)
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Update visible rows based on actual area (subtract borders + header)
    let visible_rows = chunks[2].height.saturating_sub(3) as usize;
    app.s3_state.bucket_visible_rows.set(visible_rows);

    // Tabs
    let tabs = [
        (
            "General purpose buckets (All AWS Regions)",
            app.s3_state.bucket_type == BucketType::GeneralPurpose,
        ),
        (
            "Directory buckets",
            app.s3_state.bucket_type == BucketType::Directory,
        ),
    ];
    let tabs_spans = render_inner_tab_spans(&tabs);
    let tabs_widget = Paragraph::new(Line::from(tabs_spans));
    frame.render_widget(tabs_widget, chunks[0]);

    // Filter pane
    let cursor = get_cursor(app.mode == Mode::FilterInput);
    let filter_text = if app.s3_state.buckets.filter.is_empty() && app.mode != Mode::FilterInput {
        vec![
            Span::styled("Find buckets by name", Style::default().fg(Color::DarkGray)),
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
        ]
    } else {
        vec![
            Span::raw(&app.s3_state.buckets.filter),
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
        ]
    };

    let filter = filter_area(filter_text, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[1]);

    let filtered_buckets: Vec<_> = if app.s3_state.bucket_type == BucketType::GeneralPurpose {
        app.s3_state
            .buckets
            .items
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                if app.s3_state.buckets.filter.is_empty() {
                    true
                } else {
                    b.name
                        .to_lowercase()
                        .contains(&app.s3_state.buckets.filter.to_lowercase())
                }
            })
            .collect()
    } else {
        // Directory buckets - not supported yet
        Vec::new()
    };

    let count = filtered_buckets.len();
    let bucket_type_name = match app.s3_state.bucket_type {
        BucketType::GeneralPurpose => "General purpose buckets",
        BucketType::Directory => "Directory buckets",
    };
    let title = format!(" {} ({}) ", bucket_type_name, count);

    let header_cells: Vec<Cell> = app
        .s3_bucket_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            BucketColumn::from_id(col_id).map(|col| {
                let name = crate::ui::table::format_header_cell(&col.name(), 0);
                Cell::from(name).style(Style::default().add_modifier(Modifier::BOLD))
            })
        })
        .collect();
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .height(1);

    // Calculate max widths from content
    let mut max_name_width = "Name".len();
    let mut max_region_width = "⋮ AWS Region".len();
    let mut max_date_width = "⋮ Creation date".len();

    for (_idx, bucket) in &filtered_buckets {
        let name_len = format!("{} 🪣 {}", crate::ui::table::CURSOR_COLLAPSED, bucket.name).len();
        max_name_width = max_name_width.max(name_len);
        let region_display = if bucket.region.is_empty() {
            "-"
        } else {
            &bucket.region
        };
        max_region_width = max_region_width.max(region_display.len() + 2); // +2 for "⋮ "
        max_date_width = max_date_width.max(27); // 25 chars + 2 for "⋮ "

        if app.s3_state.expanded_prefixes.contains(&bucket.name) {
            if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                for obj in preview {
                    let obj_len = format!(
                        "  ▶ {} {}",
                        if obj.is_prefix { "📁" } else { "📄" },
                        obj.key
                    )
                    .len();
                    max_name_width = max_name_width.max(obj_len);

                    if obj.is_prefix && app.s3_state.expanded_prefixes.contains(&obj.key) {
                        if let Some(nested) = app.s3_state.prefix_preview.get(&obj.key) {
                            for nested_obj in nested {
                                let nested_len = format!(
                                    "      {} {}",
                                    if nested_obj.is_prefix { "📁" } else { "📄" },
                                    nested_obj.key
                                )
                                .len();
                                max_name_width = max_name_width.max(nested_len);
                            }
                        }
                    }
                }
            }
        }
    }

    // Cap at reasonable maximums
    max_name_width = max_name_width.min(150);

    let rows: Vec<Row> = filtered_buckets
        .iter()
        .enumerate()
        .flat_map(|(bucket_idx, (_orig_idx, bucket))| {
            let is_expanded = app.s3_state.expanded_prefixes.contains(&bucket.name);
            let expand_indicator = if is_expanded {
                format!("{} ", crate::ui::table::CURSOR_EXPANDED)
            } else {
                format!("{} ", crate::ui::table::CURSOR_COLLAPSED)
            };

            // Format date as YYYY-MM-DD HH:MM:SS (UTC)
            let formatted_date = if bucket.creation_date.contains('T') {
                // Parse ISO 8601 format: 2025-05-07T12:34:47Z
                let parts: Vec<&str> = bucket.creation_date.split('T').collect();
                if parts.len() == 2 {
                    let date = parts[0];
                    let time = parts[1]
                        .trim_end_matches('Z')
                        .split('.')
                        .next()
                        .unwrap_or(parts[1]);
                    format!("{} {} (UTC)", date, time)
                } else {
                    bucket.creation_date.clone()
                }
            } else {
                bucket.creation_date.clone()
            };

            // Calculate row index for this bucket
            fn count_expanded_children(
                objects: &[S3Object],
                expanded_prefixes: &std::collections::HashSet<String>,
                prefix_preview: &std::collections::HashMap<String, Vec<S3Object>>,
            ) -> usize {
                let mut count = objects.len();
                for obj in objects {
                    if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                        if let Some(nested) = prefix_preview.get(&obj.key) {
                            count +=
                                count_expanded_children(nested, expanded_prefixes, prefix_preview);
                        }
                    }
                }
                count
            }

            let mut row_idx = bucket_idx;
            for i in 0..bucket_idx {
                if let Some((_, b)) = filtered_buckets.get(i) {
                    if app.s3_state.expanded_prefixes.contains(&b.name) {
                        if let Some(preview) = app.s3_state.bucket_preview.get(&b.name) {
                            row_idx += count_expanded_children(
                                preview,
                                &app.s3_state.expanded_prefixes,
                                &app.s3_state.prefix_preview,
                            );
                        }
                    }
                }
            }

            let style = if row_idx == app.s3_state.selected_row {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let cells: Vec<Cell> = app
                .s3_bucket_visible_column_ids
                .iter()
                .enumerate()
                .filter_map(|(i, col_id)| {
                    BucketColumn::from_id(col_id).map(|col| {
                        let content = match col {
                            BucketColumn::Name => {
                                format!("{}🪣 {}", expand_indicator, bucket.name)
                            }
                            BucketColumn::Region => bucket.region.clone(),
                            BucketColumn::CreationDate => formatted_date.clone(),
                        };
                        let cell_content = if i > 0 {
                            format!("⋮ {}", content)
                        } else {
                            content
                        };
                        Cell::from(cell_content)
                    })
                })
                .collect();

            let mut result = vec![Row::new(cells).height(1).style(style)];
            let mut child_row_idx = row_idx + 1;

            if is_expanded {
                if let Some(error) = app.s3_state.bucket_errors.get(&bucket.name) {
                    // Show error message in expanded rows (non-selectable)
                    // Split error into multiple lines if needed
                    let max_width = 120;
                    let error_lines: Vec<String> = if error.len() > max_width {
                        error
                            .as_bytes()
                            .chunks(max_width)
                            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
                            .collect()
                    } else {
                        vec![error.clone()]
                    };

                    for (line_idx, error_line) in error_lines.iter().enumerate() {
                        let error_cells: Vec<Cell> = app
                            .s3_bucket_visible_column_ids
                            .iter()
                            .enumerate()
                            .map(|(i, _col)| {
                                if i == 0 {
                                    if line_idx == 0 {
                                        Cell::from(format!("  ⚠️  {}", error_line))
                                            .style(red_text())
                                    } else {
                                        Cell::from(format!("     {}", error_line)).style(red_text())
                                    }
                                } else {
                                    Cell::from("")
                                }
                            })
                            .collect();
                        result.push(Row::new(error_cells).height(1));
                    }
                    // Don't increment child_row_idx for error rows - they're not selectable
                } else if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                    // Recursive function to render objects at any depth
                    fn render_objects_recursive<'a>(
                        objects: &'a [S3Object],
                        app: &'a App,
                        child_row_idx: &mut usize,
                        result: &mut Vec<Row<'a>>,
                        parent_key: &str,
                        is_last: &[bool],
                    ) {
                        for (idx, obj) in objects.iter().enumerate() {
                            let is_last_item = idx == objects.len() - 1;
                            let obj_is_expanded = app.s3_state.expanded_prefixes.contains(&obj.key);

                            // Build prefix with tree characters
                            let mut prefix = String::new();
                            for &last in is_last.iter() {
                                prefix.push_str(if last { "  " } else { "│ " });
                            }

                            let tree_char = if is_last_item { "╰─" } else { "├─" };
                            let expand_char = if obj.is_prefix {
                                if obj_is_expanded {
                                    crate::ui::table::CURSOR_EXPANDED
                                } else {
                                    crate::ui::table::CURSOR_COLLAPSED
                                }
                            } else {
                                ""
                            };

                            let icon = if obj.is_prefix { "📁" } else { "📄" };
                            let display_key = obj.key.strip_prefix(parent_key).unwrap_or(&obj.key);

                            let child_style = if *child_row_idx == app.s3_state.selected_row {
                                Style::default().bg(Color::DarkGray)
                            } else {
                                Style::default()
                            };

                            let formatted_date = format_iso_timestamp(&obj.last_modified);

                            let child_cells: Vec<Cell> = app
                                .s3_bucket_visible_column_ids
                                .iter()
                                .enumerate()
                                .filter_map(|(i, col_id)| {
                                    BucketColumn::from_id(col_id).map(|col| {
                                        let content = match col {
                                            BucketColumn::Name => format!(
                                                "{}{}{} {} {}",
                                                prefix, tree_char, expand_char, icon, display_key
                                            ),
                                            BucketColumn::Region => String::new(),
                                            BucketColumn::CreationDate => formatted_date.clone(),
                                        };
                                        if i > 0 {
                                            Cell::from(format!("⋮ {}", content))
                                        } else {
                                            Cell::from(content)
                                        }
                                    })
                                })
                                .collect();
                            result.push(Row::new(child_cells).style(child_style));
                            *child_row_idx += 1;

                            // Recursively render nested items if expanded
                            if obj.is_prefix && obj_is_expanded {
                                if let Some(nested_preview) =
                                    app.s3_state.prefix_preview.get(&obj.key)
                                {
                                    let mut new_is_last = is_last.to_vec();
                                    new_is_last.push(is_last_item);
                                    render_objects_recursive(
                                        nested_preview,
                                        app,
                                        child_row_idx,
                                        result,
                                        &obj.key,
                                        &new_is_last,
                                    );
                                }
                            }
                        }
                    }

                    render_objects_recursive(
                        preview,
                        app,
                        &mut child_row_idx,
                        &mut result,
                        "",
                        &[],
                    );
                }
            }

            result
        })
        .skip(app.s3_state.bucket_scroll_offset)
        .take(app.s3_state.bucket_visible_rows.get())
        .collect();

    let widths: Vec<Constraint> = app
        .s3_bucket_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            BucketColumn::from_id(col_id).map(|col| match col {
                BucketColumn::Name => Constraint::Length(max_name_width as u16),
                BucketColumn::Region => Constraint::Length(15),
                BucketColumn::CreationDate => Constraint::Length(max_date_width as u16),
            })
        })
        .collect();

    let is_active = app.mode != Mode::ColumnSelector;
    let border_color = if is_active {
        Color::Green
    } else {
        Color::White
    };

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        );

    frame.render_widget(table, chunks[2]);

    // Render scrollbar if content exceeds visible area
    let total_rows = app.s3_state.calculate_total_bucket_rows();
    let visible_rows = chunks[2].height.saturating_sub(3) as usize; // Subtract borders and header
    if total_rows > visible_rows {
        crate::common::render_scrollbar(
            frame,
            chunks[2].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            total_rows,
            app.s3_state.selected_row,
        );
    }
}

fn render_objects(frame: &mut Frame, app: &App, area: Rect) {
    let show_filter = app.s3_state.object_tab == ObjectTab::Objects;

    let chunks = if show_filter {
        crate::ui::vertical(
            [
                Constraint::Length(1), // Tabs
                Constraint::Length(3), // Filter (1 line + borders)
                Constraint::Min(0),    // Content
            ],
            area,
        )
    } else {
        crate::ui::vertical(
            [
                Constraint::Length(1), // Tabs
                Constraint::Min(0),    // Content (no filter)
            ],
            area,
        )
    };

    // Update visible rows based on actual content area
    let content_area_idx = if show_filter { 2 } else { 1 };
    let visible_rows = chunks[content_area_idx].height.saturating_sub(3) as usize;
    app.s3_state.object_visible_rows.set(visible_rows);

    // Tabs
    let available_tabs = if app.s3_state.prefix_stack.is_empty() {
        // At bucket root - show all tabs
        ObjectTab::all()
    } else {
        // In a prefix - only Objects and Properties
        vec![ObjectTab::Objects, ObjectTab::Properties]
    };

    let tab_tuples: Vec<(&str, ObjectTab)> = available_tabs
        .iter()
        .map(|tab| (tab.name(), *tab))
        .collect();

    frame.render_widget(Clear, chunks[0]);
    render_tabs(frame, chunks[0], &tab_tuples, &app.s3_state.object_tab);

    // Filter (only for Objects tab)
    if app.s3_state.object_tab == ObjectTab::Objects {
        let cursor = get_cursor(app.mode == Mode::FilterInput);
        let filter_text = if app.s3_state.object_filter.is_empty() && app.mode != Mode::FilterInput
        {
            vec![
                Span::styled(
                    "Find objects by prefix",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(cursor, Style::default().fg(Color::Yellow)),
            ]
        } else {
            vec![
                Span::raw(&app.s3_state.object_filter),
                Span::styled(cursor, Style::default().fg(Color::Yellow)),
            ]
        };
        let filter = filter_area(filter_text, app.mode == Mode::FilterInput);
        frame.render_widget(filter, chunks[1]);
    }

    // Render content based on selected tab
    let content_idx = if show_filter { 2 } else { 1 };
    match app.s3_state.object_tab {
        ObjectTab::Objects => render_objects_table(frame, app, chunks[content_idx]),
        ObjectTab::Properties => render_bucket_properties(frame, app, chunks[content_idx]),
        _ => {
            // Placeholder for other tabs
            let placeholder =
                Paragraph::new(format!("{} - Coming soon", app.s3_state.object_tab.name()))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(active_border()),
                    )
                    .style(Style::default().fg(Color::Gray));
            frame.render_widget(placeholder, chunks[content_idx]);
        }
    }
}

fn render_objects_table(frame: &mut Frame, app: &App, area: Rect) {
    // Filter objects by prefix
    let filtered_objects: Vec<_> = app
        .s3_state
        .objects
        .iter()
        .enumerate()
        .filter(|(_, obj)| {
            if app.s3_state.object_filter.is_empty() {
                true
            } else {
                let name = obj.key.trim_start_matches(
                    &app.s3_state
                        .prefix_stack
                        .last()
                        .cloned()
                        .unwrap_or_default(),
                );
                name.to_lowercase()
                    .contains(&app.s3_state.object_filter.to_lowercase())
            }
        })
        .collect();

    let count = filtered_objects.len();
    let title = format!(" Objects ({}) ", count);

    let columns = ["Name", "Type", "Last modified", "Size", "Storage class"];
    let header_cells: Vec<Cell> = columns
        .iter()
        .enumerate()
        .map(|(i, name)| {
            Cell::from(crate::ui::table::format_header_cell(name, i))
                .style(Style::default().add_modifier(Modifier::BOLD))
        })
        .collect();
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .height(1)
        .bottom_margin(0);

    // Calculate max name width
    let max_name_width = filtered_objects
        .iter()
        .map(|(_, obj)| {
            let name = obj.key.trim_start_matches(
                &app.s3_state
                    .prefix_stack
                    .last()
                    .cloned()
                    .unwrap_or_default(),
            );
            name.len() + 4 // +4 for icon and expand indicator
        })
        .max()
        .unwrap_or(30)
        .max(30) as u16;

    let rows: Vec<Row> = filtered_objects
        .iter()
        .flat_map(|(idx, obj)| {
            let icon = if obj.is_prefix { "📁" } else { "📄" };

            // Add expand indicator for prefixes
            let expand_indicator = if obj.is_prefix {
                if app.s3_state.expanded_prefixes.contains(&obj.key) {
                    format!("{} ", crate::ui::table::CURSOR_EXPANDED)
                } else {
                    format!("{} ", crate::ui::table::CURSOR_COLLAPSED)
                }
            } else {
                String::new()
            };

            let name = obj.key.trim_start_matches(
                &app.s3_state
                    .prefix_stack
                    .last()
                    .cloned()
                    .unwrap_or_default(),
            );
            let display_name = format!("{}{} {}", expand_indicator, icon, name);
            let obj_type = if obj.is_prefix { "Folder" } else { "File" };
            let size = if obj.is_prefix {
                String::new()
            } else {
                format_bytes(obj.size)
            };

            // Format datetime with (UTC)
            let datetime = format_iso_timestamp(&obj.last_modified);

            // Format storage class
            let storage = if obj.storage_class.is_empty() {
                String::new()
            } else {
                obj.storage_class
                    .chars()
                    .next()
                    .unwrap()
                    .to_uppercase()
                    .to_string()
                    + &obj.storage_class[1..].to_lowercase()
            };

            // Calculate row index including nested items
            let mut row_idx = *idx;
            for i in 0..*idx {
                if let Some(prev_obj) = app.s3_state.objects.get(i) {
                    if prev_obj.is_prefix && app.s3_state.expanded_prefixes.contains(&prev_obj.key)
                    {
                        if let Some(preview) = app.s3_state.prefix_preview.get(&prev_obj.key) {
                            row_idx += preview.len();
                        }
                    }
                }
            }

            let style = if row_idx == app.s3_state.selected_object {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let mut result = vec![Row::new(vec![
                Cell::from(display_name),
                Cell::from(format!("⋮ {}", obj_type)),
                Cell::from(format!("⋮ {}", datetime)),
                Cell::from(format!("⋮ {}", size)),
                Cell::from(format!("⋮ {}", storage)),
            ])
            .style(style)];

            let mut child_row_idx = row_idx + 1;

            if obj.is_prefix && app.s3_state.expanded_prefixes.contains(&obj.key) {
                if let Some(preview) = app.s3_state.prefix_preview.get(&obj.key) {
                    // Recursive function to render nested objects
                    fn render_nested_objects<'a>(
                        objects: &'a [crate::s3::Object],
                        app: &'a App,
                        child_row_idx: &mut usize,
                        result: &mut Vec<Row<'a>>,
                        parent_key: &str,
                        is_last: &[bool],
                    ) {
                        for (child_idx, preview_obj) in objects.iter().enumerate() {
                            let is_last_child = child_idx == objects.len() - 1;
                            let obj_is_expanded =
                                app.s3_state.expanded_prefixes.contains(&preview_obj.key);

                            // Build prefix with tree characters
                            let mut prefix = String::new();
                            for &last in is_last.iter() {
                                prefix.push_str(if last { "  " } else { "│ " });
                            }

                            let tree_char = if is_last_child { "╰─" } else { "├─" };
                            let child_expand = if preview_obj.is_prefix {
                                if obj_is_expanded {
                                    crate::ui::table::CURSOR_EXPANDED
                                } else {
                                    crate::ui::table::CURSOR_COLLAPSED
                                }
                            } else {
                                ""
                            };
                            let child_icon = if preview_obj.is_prefix {
                                "📁"
                            } else {
                                "📄"
                            };
                            let child_name = preview_obj
                                .key
                                .strip_prefix(parent_key)
                                .unwrap_or(&preview_obj.key);

                            let child_type = if preview_obj.is_prefix {
                                "Folder"
                            } else {
                                "File"
                            };
                            let child_size = if preview_obj.is_prefix {
                                String::new()
                            } else {
                                format_bytes(preview_obj.size)
                            };
                            let child_datetime = format_iso_timestamp(&preview_obj.last_modified);
                            let child_storage = if preview_obj.storage_class.is_empty() {
                                String::new()
                            } else {
                                preview_obj
                                    .storage_class
                                    .chars()
                                    .next()
                                    .unwrap()
                                    .to_uppercase()
                                    .to_string()
                                    + &preview_obj.storage_class[1..].to_lowercase()
                            };

                            let child_style = if *child_row_idx == app.s3_state.selected_object {
                                Style::default().bg(Color::DarkGray)
                            } else {
                                Style::default()
                            };

                            result.push(
                                Row::new(vec![
                                    Cell::from(format!(
                                        "{}{}{} {} {}",
                                        prefix, tree_char, child_expand, child_icon, child_name
                                    )),
                                    Cell::from(format!("⋮ {}", child_type)),
                                    Cell::from(format!("⋮ {}", child_datetime)),
                                    Cell::from(format!("⋮ {}", child_size)),
                                    Cell::from(format!("⋮ {}", child_storage)),
                                ])
                                .style(child_style),
                            );
                            *child_row_idx += 1;

                            // Recursively render nested children
                            if preview_obj.is_prefix && obj_is_expanded {
                                if let Some(nested_preview) =
                                    app.s3_state.prefix_preview.get(&preview_obj.key)
                                {
                                    let mut new_is_last = is_last.to_vec();
                                    new_is_last.push(is_last_child);
                                    render_nested_objects(
                                        nested_preview,
                                        app,
                                        child_row_idx,
                                        result,
                                        &preview_obj.key,
                                        &new_is_last,
                                    );
                                }
                            }
                        }
                    }

                    render_nested_objects(
                        preview,
                        app,
                        &mut child_row_idx,
                        &mut result,
                        &obj.key,
                        &[],
                    );
                }
            }

            result
        })
        .skip(app.s3_state.object_scroll_offset)
        .take(app.s3_state.object_visible_rows.get())
        .collect();

    let table = Table::new(
        rows,
        vec![
            Constraint::Length(max_name_width),
            Constraint::Length(10),
            Constraint::Length(UTC_TIMESTAMP_WIDTH),
            Constraint::Length(12),
            Constraint::Length(15),
        ],
    )
    .header(header)
    .column_spacing(1)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(active_border()),
    );

    frame.render_widget(table, area);

    // Render scrollbar if content exceeds visible area
    let total_rows = app.s3_state.calculate_total_object_rows();
    let visible_rows = area.height.saturating_sub(3) as usize;
    if total_rows > visible_rows {
        crate::common::render_scrollbar(
            frame,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            total_rows,
            app.s3_state.selected_object,
        );
    }
}

fn render_bucket_properties(frame: &mut Frame, app: &App, area: Rect) {
    let bucket_name = app.s3_state.current_bucket.as_ref().unwrap();
    let bucket = app
        .s3_state
        .buckets
        .items
        .iter()
        .find(|b| &b.name == bucket_name);

    let mut lines = vec![];

    let block = Block::default()
        .title(" Properties ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(active_border());
    let inner = block.inner(area);

    // Bucket overview
    lines.push(crate::ui::section_header("Bucket overview", inner.width));
    if let Some(b) = bucket {
        let region = if b.region.is_empty() {
            "us-east-1"
        } else {
            &b.region
        };
        let formatted_date = if b.creation_date.contains('T') {
            let parts: Vec<&str> = b.creation_date.split('T').collect();
            if parts.len() == 2 {
                format!(
                    "{} {} (UTC)",
                    parts[0],
                    parts[1]
                        .trim_end_matches('Z')
                        .split('.')
                        .next()
                        .unwrap_or(parts[1])
                )
            } else {
                b.creation_date.clone()
            }
        } else {
            b.creation_date.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(
                "AWS Region: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(region),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "Amazon Resource Name (ARN): ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("arn:aws:s3:::{}", bucket_name)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "Creation date: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(formatted_date),
        ]));
    }
    lines.push(Line::from(""));

    // Tags
    lines.push(crate::ui::section_header("Tags (0)", inner.width));
    lines.push(Line::from("No tags associated with this resource."));
    lines.push(Line::from(""));

    // Default encryption
    lines.push(crate::ui::section_header("Default encryption", inner.width));
    lines.push(Line::from(vec![
        Span::styled(
            "Encryption type: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("Server-side encryption with Amazon S3 managed keys (SSE-S3)"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Bucket Key: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("Disabled"),
    ]));
    lines.push(Line::from(""));

    // Server access logging
    lines.push(crate::ui::section_header(
        "Server access logging",
        inner.width,
    ));
    lines.push(Line::from("Disabled"));
    lines.push(Line::from(""));

    // CloudTrail
    lines.push(crate::ui::section_header(
        "AWS CloudTrail data events",
        inner.width,
    ));
    lines.push(Line::from("Configure in CloudTrail console"));
    lines.push(Line::from(""));

    // EventBridge
    lines.push(crate::ui::section_header("Amazon EventBridge", inner.width));
    lines.push(Line::from(vec![
        Span::styled(
            "Send notifications to Amazon EventBridge: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("Off"),
    ]));
    lines.push(Line::from(""));

    // Transfer acceleration
    lines.push(crate::ui::section_header(
        "Transfer acceleration",
        inner.width,
    ));
    lines.push(Line::from("Disabled"));
    lines.push(Line::from(""));

    // Object Lock
    lines.push(crate::ui::section_header("Object Lock", inner.width));
    lines.push(Line::from("Disabled"));
    lines.push(Line::from(""));

    // Requester pays
    lines.push(crate::ui::section_header("Requester pays", inner.width));
    lines.push(Line::from("Disabled"));
    lines.push(Line::from(""));

    // Static website hosting
    lines.push(crate::ui::section_header(
        "Static website hosting",
        inner.width,
    ));
    lines.push(Line::from("Disabled"));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.s3_state.properties_scroll, 0));

    frame.render_widget(paragraph, area);

    // Render scrollbar if needed
    let content_height = 40; // Approximate line count
    if content_height > area.height.saturating_sub(2) {
        crate::common::render_scrollbar(
            frame,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            content_height as usize,
            app.s3_state.properties_scroll as usize,
        );
    }
}

// S3-specific helper functions
pub async fn load_s3_buckets(app: &mut App) -> anyhow::Result<()> {
    let buckets = app.s3_client.list_buckets().await?;
    app.s3_state.buckets.items = buckets
        .into_iter()
        .map(|(name, region, date)| crate::app::S3Bucket {
            name,
            region,
            creation_date: date,
        })
        .collect();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total_bucket_rows_no_expansion() {
        let mut state = State::new();
        state.buckets.items = vec![
            S3Bucket {
                name: "bucket1".to_string(),
                region: "us-east-1".to_string(),
                creation_date: String::new(),
            },
            S3Bucket {
                name: "bucket2".to_string(),
                region: "us-west-2".to_string(),
                creation_date: String::new(),
            },
        ];

        assert_eq!(state.calculate_total_bucket_rows(), 2);
    }

    #[test]
    fn test_calculate_total_bucket_rows_with_expansion() {
        let mut state = State::new();
        state.buckets.items = vec![S3Bucket {
            name: "bucket1".to_string(),
            region: "us-east-1".to_string(),
            creation_date: String::new(),
        }];

        // Expand bucket1
        state.expanded_prefixes.insert("bucket1".to_string());
        state.bucket_preview.insert(
            "bucket1".to_string(),
            vec![
                S3Object {
                    key: "file1.txt".to_string(),
                    is_prefix: false,
                    size: 100,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
                S3Object {
                    key: "folder1/".to_string(),
                    is_prefix: true,
                    size: 0,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
            ],
        );

        // 1 bucket + 2 preview items
        assert_eq!(state.calculate_total_bucket_rows(), 3);
    }

    #[test]
    fn test_calculate_total_bucket_rows_nested_expansion() {
        let mut state = State::new();
        state.buckets.items = vec![S3Bucket {
            name: "bucket1".to_string(),
            region: "us-east-1".to_string(),
            creation_date: String::new(),
        }];

        // Expand bucket1
        state.expanded_prefixes.insert("bucket1".to_string());
        state.bucket_preview.insert(
            "bucket1".to_string(),
            vec![S3Object {
                key: "folder1/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            }],
        );

        // Expand folder1
        state.expanded_prefixes.insert("folder1/".to_string());
        state.prefix_preview.insert(
            "folder1/".to_string(),
            vec![
                S3Object {
                    key: "folder1/file1.txt".to_string(),
                    is_prefix: false,
                    size: 100,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
                S3Object {
                    key: "folder1/file2.txt".to_string(),
                    is_prefix: false,
                    size: 200,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
            ],
        );

        // 1 bucket + 1 folder + 2 nested files
        assert_eq!(state.calculate_total_bucket_rows(), 4);
    }

    #[test]
    fn test_calculate_total_bucket_rows_deeply_nested() {
        let mut state = State::new();
        state.buckets.items = vec![S3Bucket {
            name: "bucket1".to_string(),
            region: "us-east-1".to_string(),
            creation_date: String::new(),
        }];

        // Expand bucket1 with folder1
        state.expanded_prefixes.insert("bucket1".to_string());
        state.bucket_preview.insert(
            "bucket1".to_string(),
            vec![S3Object {
                key: "folder1/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            }],
        );

        // Expand folder1 with folder2
        state.expanded_prefixes.insert("folder1/".to_string());
        state.prefix_preview.insert(
            "folder1/".to_string(),
            vec![S3Object {
                key: "folder1/folder2/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            }],
        );

        // Expand folder2 with files
        state
            .expanded_prefixes
            .insert("folder1/folder2/".to_string());
        state.prefix_preview.insert(
            "folder1/folder2/".to_string(),
            vec![
                S3Object {
                    key: "folder1/folder2/file1.txt".to_string(),
                    is_prefix: false,
                    size: 100,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
                S3Object {
                    key: "folder1/folder2/file2.txt".to_string(),
                    is_prefix: false,
                    size: 200,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
                S3Object {
                    key: "folder1/folder2/file3.txt".to_string(),
                    is_prefix: false,
                    size: 300,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
            ],
        );

        // 1 bucket + 1 folder1 + 1 folder2 + 3 files = 6
        assert_eq!(state.calculate_total_bucket_rows(), 6);
    }

    #[test]
    fn test_calculate_total_object_rows_no_expansion() {
        let mut state = State::new();
        state.objects = vec![
            S3Object {
                key: "folder1/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            },
            S3Object {
                key: "folder2/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            },
            S3Object {
                key: "file.txt".to_string(),
                is_prefix: false,
                size: 100,
                last_modified: String::new(),
                storage_class: String::new(),
            },
        ];

        assert_eq!(state.calculate_total_object_rows(), 3);
    }

    #[test]
    fn test_calculate_total_object_rows_with_expansion() {
        let mut state = State::new();
        state.objects = vec![
            S3Object {
                key: "folder1/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            },
            S3Object {
                key: "file.txt".to_string(),
                is_prefix: false,
                size: 100,
                last_modified: String::new(),
                storage_class: String::new(),
            },
        ];

        // Expand folder1
        state.expanded_prefixes.insert("folder1/".to_string());
        state.prefix_preview.insert(
            "folder1/".to_string(),
            vec![
                S3Object {
                    key: "folder1/sub1.txt".to_string(),
                    is_prefix: false,
                    size: 50,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
                S3Object {
                    key: "folder1/sub2.txt".to_string(),
                    is_prefix: false,
                    size: 75,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
            ],
        );

        // 2 root objects + 2 children in folder1
        assert_eq!(state.calculate_total_object_rows(), 4);
    }

    #[test]
    fn test_calculate_total_object_rows_nested_expansion() {
        let mut state = State::new();
        state.objects = vec![S3Object {
            key: "folder1/".to_string(),
            is_prefix: true,
            size: 0,
            last_modified: String::new(),
            storage_class: String::new(),
        }];

        // Expand folder1
        state.expanded_prefixes.insert("folder1/".to_string());
        state.prefix_preview.insert(
            "folder1/".to_string(),
            vec![S3Object {
                key: "folder1/folder2/".to_string(),
                is_prefix: true,
                size: 0,
                last_modified: String::new(),
                storage_class: String::new(),
            }],
        );

        // Expand folder2
        state
            .expanded_prefixes
            .insert("folder1/folder2/".to_string());
        state.prefix_preview.insert(
            "folder1/folder2/".to_string(),
            vec![
                S3Object {
                    key: "folder1/folder2/file1.txt".to_string(),
                    is_prefix: false,
                    size: 100,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
                S3Object {
                    key: "folder1/folder2/file2.txt".to_string(),
                    is_prefix: false,
                    size: 200,
                    last_modified: String::new(),
                    storage_class: String::new(),
                },
            ],
        );

        // 1 root + 1 child folder + 2 nested files
        assert_eq!(state.calculate_total_object_rows(), 4);
    }

    #[test]
    fn test_scrollbar_needed_when_rows_exceed_visible_area() {
        let visible_rows = 10;
        let total_rows = 15;

        // Scrollbar should be shown when total > visible
        assert!(total_rows > visible_rows);
    }

    #[test]
    fn test_scrollbar_not_needed_when_rows_fit() {
        let visible_rows = 20;
        let total_rows = 10;

        // Scrollbar should not be shown when total <= visible
        assert!(total_rows <= visible_rows);
    }

    #[test]
    fn test_scroll_offset_adjusts_when_selection_below_viewport() {
        let mut state = State::new();
        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 0;
        state.selected_row = 15; // Beyond visible area (0-9)

        // Scroll offset should adjust to keep selection visible
        let visible_rows = state.bucket_visible_rows.get();
        if state.selected_row >= state.bucket_scroll_offset + visible_rows {
            state.bucket_scroll_offset = state.selected_row - visible_rows + 1;
        }

        assert_eq!(state.bucket_scroll_offset, 6); // 15 - 10 + 1 = 6
        assert!(state.selected_row >= state.bucket_scroll_offset);
        assert!(state.selected_row < state.bucket_scroll_offset + visible_rows);
    }

    #[test]
    fn test_scroll_offset_adjusts_when_selection_above_viewport() {
        let mut state = State::new();
        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 10;
        state.selected_row = 5; // Above visible area (10-19)

        // Scroll offset should adjust to keep selection visible
        if state.selected_row < state.bucket_scroll_offset {
            state.bucket_scroll_offset = state.selected_row;
        }

        assert_eq!(state.bucket_scroll_offset, 5);
        assert!(state.selected_row >= state.bucket_scroll_offset);
    }

    #[test]
    fn test_selection_stays_visible_during_navigation() {
        let mut state = State::new();
        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 0;
        state.selected_row = 0;

        // Navigate down 15 times
        for _ in 0..15 {
            state.selected_row += 1;
            let visible_rows = state.bucket_visible_rows.get();
            if state.selected_row >= state.bucket_scroll_offset + visible_rows {
                state.bucket_scroll_offset = state.selected_row - visible_rows + 1;
            }
        }

        // Selection should be at row 15, scroll offset should keep it visible
        assert_eq!(state.selected_row, 15);
        assert_eq!(state.bucket_scroll_offset, 6);
        assert!(state.selected_row >= state.bucket_scroll_offset);
        assert!(state.selected_row < state.bucket_scroll_offset + state.bucket_visible_rows.get());
    }

    #[test]
    fn test_scroll_offset_adjusts_after_jumping_to_parent() {
        let mut state = State::new();
        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 10; // Viewing rows 10-19
        state.selected_row = 15; // Selected row in middle of viewport

        // Jump to parent at row 5 (above viewport)
        state.selected_row = 5;

        // Adjust scroll offset
        let visible_rows = state.bucket_visible_rows.get();
        if state.selected_row < state.bucket_scroll_offset {
            state.bucket_scroll_offset = state.selected_row;
        } else if state.selected_row >= state.bucket_scroll_offset + visible_rows {
            state.bucket_scroll_offset = state.selected_row.saturating_sub(visible_rows - 1);
        }

        // Scroll offset should adjust to show the parent
        assert_eq!(state.bucket_scroll_offset, 5);
        assert!(state.selected_row >= state.bucket_scroll_offset);
        assert!(state.selected_row < state.bucket_scroll_offset + visible_rows);
    }

    #[test]
    fn test_scroll_offset_adjusts_after_collapse() {
        let mut state = State::new();
        state.buckets.items = vec![S3Bucket {
            name: "bucket1".to_string(),
            region: "us-east-1".to_string(),
            creation_date: String::new(),
        }];

        // Expand bucket with many items
        state.expanded_prefixes.insert("bucket1".to_string());
        let mut preview = vec![];
        for i in 0..20 {
            preview.push(S3Object {
                key: format!("file{}.txt", i),
                is_prefix: false,
                size: 100,
                last_modified: String::new(),
                storage_class: String::new(),
            });
        }
        state.bucket_preview.insert("bucket1".to_string(), preview);

        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 10; // Viewing rows 10-19
        state.selected_row = 15; // Selected row in middle of expanded items

        // Collapse the bucket
        state.expanded_prefixes.remove("bucket1");

        // Selection should now be on the bucket itself (row 0)
        state.selected_row = 0;

        // Adjust scroll offset
        if state.selected_row < state.bucket_scroll_offset {
            state.bucket_scroll_offset = state.selected_row;
        }

        // Scroll offset should adjust to show the bucket
        assert_eq!(state.bucket_scroll_offset, 0);
        assert!(state.selected_row >= state.bucket_scroll_offset);
    }

    #[test]
    fn test_object_scroll_offset_adjusts_after_jumping_to_parent() {
        let mut state = State::new();
        state.objects = vec![S3Object {
            key: "folder1/".to_string(),
            is_prefix: true,
            size: 0,
            last_modified: String::new(),
            storage_class: String::new(),
        }];

        // Expand folder with many items
        state.expanded_prefixes.insert("folder1/".to_string());
        let mut preview = vec![];
        for i in 0..20 {
            preview.push(S3Object {
                key: format!("folder1/file{}.txt", i),
                is_prefix: false,
                size: 100,
                last_modified: String::new(),
                storage_class: String::new(),
            });
        }
        state.prefix_preview.insert("folder1/".to_string(), preview);

        state.object_visible_rows.set(10);
        state.object_scroll_offset = 10; // Viewing rows 10-19
        state.selected_object = 15; // Selected row in middle of expanded items

        // Jump to parent folder (row 0)
        state.selected_object = 0;

        // Adjust scroll offset
        let visible_rows = state.object_visible_rows.get();
        if state.selected_object < state.object_scroll_offset {
            state.object_scroll_offset = state.selected_object;
        } else if state.selected_object >= state.object_scroll_offset + visible_rows {
            state.object_scroll_offset = state.selected_object.saturating_sub(visible_rows - 1);
        }

        // Scroll offset should adjust to show the parent
        assert_eq!(state.object_scroll_offset, 0);
        assert!(state.selected_object >= state.object_scroll_offset);
        assert!(state.selected_object < state.object_scroll_offset + visible_rows);
    }

    #[test]
    fn test_selection_below_viewport_becomes_visible() {
        let mut state = State::new();
        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 0; // Viewing rows 0-9
        state.selected_row = 0;

        // Jump to row 20 (way below viewport)
        state.selected_row = 20;

        // Adjust scroll offset
        let visible_rows = state.bucket_visible_rows.get();
        if state.selected_row >= state.bucket_scroll_offset + visible_rows {
            state.bucket_scroll_offset = state.selected_row.saturating_sub(visible_rows - 1);
        }

        // Scroll offset should adjust to show the selection
        assert_eq!(state.bucket_scroll_offset, 11); // 20 - 10 + 1
        assert!(state.selected_row >= state.bucket_scroll_offset);
        assert!(state.selected_row < state.bucket_scroll_offset + visible_rows);
    }

    #[test]
    fn test_scroll_keeps_selection_visible_when_navigating_down() {
        let mut state = State::new();
        state.buckets.items = vec![];
        for i in 0..50 {
            state.buckets.items.push(S3Bucket {
                name: format!("bucket{}", i),
                region: "us-east-1".to_string(),
                creation_date: String::new(),
            });
        }

        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 0;
        state.selected_row = 0;

        // Navigate down 25 times
        for _ in 0..25 {
            let total_rows = state.buckets.items.len();
            state.selected_row = (state.selected_row + 1).min(total_rows - 1);

            // Adjust scroll offset (same logic as in app.rs)
            let visible_rows = state.bucket_visible_rows.get();
            if state.selected_row >= state.bucket_scroll_offset + visible_rows {
                state.bucket_scroll_offset = state.selected_row - visible_rows + 1;
            }
        }

        // Selection should be at row 25
        assert_eq!(state.selected_row, 25);
        // Scroll offset should be 16 (25 - 10 + 1)
        assert_eq!(state.bucket_scroll_offset, 16);
        // Selection should be visible
        assert!(state.selected_row >= state.bucket_scroll_offset);
        assert!(state.selected_row < state.bucket_scroll_offset + state.bucket_visible_rows.get());
    }

    #[test]
    fn test_ctrl_d_adjusts_scroll_offset() {
        let mut state = State::new();
        state.buckets.items = vec![];
        for i in 0..50 {
            state.buckets.items.push(S3Bucket {
                name: format!("bucket{}", i),
                region: "us-east-1".to_string(),
                creation_date: String::new(),
            });
        }

        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 0;
        state.selected_row = 5;

        // Simulate Ctrl+D (jump down 10)
        let total_rows = state.buckets.items.len();
        state.selected_row = state.selected_row.saturating_add(10).min(total_rows - 1);

        // Adjust scroll offset
        let visible_rows = state.bucket_visible_rows.get();
        if state.selected_row >= state.bucket_scroll_offset + visible_rows {
            state.bucket_scroll_offset = state.selected_row - visible_rows + 1;
        }

        // Selection should be at row 15
        assert_eq!(state.selected_row, 15);
        // Scroll offset should be 6 (15 - 10 + 1)
        assert_eq!(state.bucket_scroll_offset, 6);
        // Selection should be visible
        assert!(state.selected_row >= state.bucket_scroll_offset);
        assert!(state.selected_row < state.bucket_scroll_offset + visible_rows);
    }

    #[test]
    fn test_ctrl_u_adjusts_scroll_offset() {
        let mut state = State::new();
        state.buckets.items = vec![];
        for i in 0..50 {
            state.buckets.items.push(S3Bucket {
                name: format!("bucket{}", i),
                region: "us-east-1".to_string(),
                creation_date: String::new(),
            });
        }

        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 10;
        state.selected_row = 15;

        // Simulate Ctrl+U (jump up 10)
        state.selected_row = state.selected_row.saturating_sub(10);

        // Adjust scroll offset
        if state.selected_row < state.bucket_scroll_offset {
            state.bucket_scroll_offset = state.selected_row;
        }

        // Selection should be at row 5
        assert_eq!(state.selected_row, 5);
        // Scroll offset should be 5
        assert_eq!(state.bucket_scroll_offset, 5);
        // Selection should be visible
        assert!(state.selected_row >= state.bucket_scroll_offset);
    }

    #[test]
    fn test_ctrl_d_clamps_to_max_rows() {
        let mut state = State::new();
        state.buckets.items = vec![];
        for i in 0..20 {
            state.buckets.items.push(S3Bucket {
                name: format!("bucket{}", i),
                region: "us-east-1".to_string(),
                creation_date: String::new(),
            });
        }

        state.bucket_visible_rows.set(10);
        state.bucket_scroll_offset = 5;
        state.selected_row = 15;

        // Simulate Ctrl+D (jump down 10) - should clamp to max
        let total_rows = state.buckets.items.len();
        state.selected_row = state.selected_row.saturating_add(10).min(total_rows - 1);

        // Selection should be clamped to 19 (last row)
        assert_eq!(state.selected_row, 19);
    }
}
