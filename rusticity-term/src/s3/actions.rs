use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::s3::BucketType as S3BucketType;

const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        app.s3_state.selected_object = 0;
    } else {
        app.s3_state.buckets.reset();
        app.s3_state.selected_row = 0;
        app.s3_state.bucket_scroll_offset = 0;
    }
}

pub fn reset_on_service_select(app: &mut App) {
    app.s3_state.selected_row = 0;
    app.s3_state.selected_object = 0;
}

pub fn start_filter(app: &mut App) {
    app.s3_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    app.s3_state.input_focus = app.s3_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.s3_state.input_focus = app.s3_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.s3_state.input_focus == InputFocus::Pagination
}

// ── Navigation — pagination ───────────────────────────────────────────────────

pub fn page_down_filter_input(app: &mut App) {
    if app.s3_state.input_focus == InputFocus::Pagination {
        bucket_page_jump(app, true);
        app.s3_state.bucket_scroll_offset = app.s3_state.selected_row;
    }
}

pub fn page_up_filter_input(app: &mut App) {
    if app.s3_state.input_focus == InputFocus::Pagination {
        bucket_page_jump(app, false);
        app.s3_state.bucket_scroll_offset = app.s3_state.selected_row;
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn go_back(app: &mut App) {
    if !app.s3_state.prefix_stack.is_empty() {
        app.s3_state.prefix_stack.pop();
        app.s3_state.buckets.loading = true;
    } else {
        app.s3_state.current_bucket = None;
        app.s3_state.objects.clear();
    }
}

pub fn next_detail_tab(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        app.s3_state.object_tab = app.s3_state.object_tab.next();
    } else {
        app.s3_state.bucket_type = match app.s3_state.bucket_type {
            S3BucketType::GeneralPurpose => S3BucketType::Directory,
            S3BucketType::Directory => S3BucketType::GeneralPurpose,
        };
        app.s3_state.buckets.reset();
    }
}

pub fn prev_detail_tab(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        app.s3_state.object_tab = app.s3_state.object_tab.prev();
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if idx > 0 && idx <= app.s3_bucket_column_ids.len() {
        if let Some(col) = app.s3_bucket_column_ids.get(idx - 1) {
            App::toggle_column_visibility(
                &mut app.s3_bucket_visible_column_ids,
                &app.s3_bucket_column_ids,
                *col,
            );
        }
    } else if idx == app.s3_bucket_column_ids.len() + 3 {
        app.s3_state.buckets.page_size = PageSize::Ten;
    } else if idx == app.s3_bucket_column_ids.len() + 4 {
        app.s3_state.buckets.page_size = PageSize::TwentyFive;
    } else if idx == app.s3_bucket_column_ids.len() + 5 {
        app.s3_state.buckets.page_size = PageSize::Fifty;
    } else if idx == app.s3_bucket_column_ids.len() + 6 {
        app.s3_state.buckets.page_size = PageSize::OneHundred;
    }
}

pub fn next_preferences(app: &mut App) {
    let page_size_idx = app.s3_bucket_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let page_size_idx = app.s3_bucket_column_ids.len() + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    app.s3_bucket_column_ids.len() + 6
}

pub fn column_count(app: &App) -> usize {
    app.s3_bucket_column_ids.len()
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["S3".to_string()];
    if let Some(bucket) = &app.s3_state.current_bucket {
        parts.push(bucket.clone());
        if let Some(prefix) = app.s3_state.prefix_stack.last() {
            parts.push(prefix.trim_end_matches('/').to_string());
        }
    } else {
        parts.push("Buckets".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::s3;
    if let Some(bucket_name) = &app.s3_state.current_bucket {
        let prefix = app.s3_state.prefix_stack.join("");
        s3::console_url_bucket(&app.config.region, bucket_name, &prefix)
    } else {
        s3::console_url_buckets(&app.config.region)
    }
}

// ── Filter active ─────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if app.s3_state.current_bucket.is_some() {
        Some(&mut app.s3_state.object_filter)
    } else {
        Some(&mut app.s3_state.buckets.filter)
    }
}

// ── Navigation — next / prev item ────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        if app.s3_state.object_tab == crate::ui::s3::ObjectTab::Properties {
            app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_add(1);
        } else {
            let total_rows = crate::ui::s3::calculate_total_object_rows(app);
            let max = total_rows.saturating_sub(1);
            app.s3_state.selected_object = (app.s3_state.selected_object + 1).min(max);
            let visible_rows = app.s3_state.object_visible_rows.get();
            if app.s3_state.selected_object >= app.s3_state.object_scroll_offset + visible_rows {
                app.s3_state.object_scroll_offset = app.s3_state.selected_object - visible_rows + 1;
            }
        }
    } else {
        let total_rows = crate::ui::s3::calculate_filtered_bucket_rows(app);
        if total_rows > 0 {
            app.s3_state.selected_row = (app.s3_state.selected_row + 1).min(total_rows - 1);
            let visible_rows = app.s3_state.bucket_visible_rows.get();
            if app.s3_state.selected_row >= app.s3_state.bucket_scroll_offset + visible_rows {
                app.s3_state.bucket_scroll_offset = app.s3_state.selected_row - visible_rows + 1;
            }
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        if app.s3_state.object_tab == crate::ui::s3::ObjectTab::Properties {
            app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_sub(1);
        } else {
            app.s3_state.selected_object = app.s3_state.selected_object.saturating_sub(1);
            if app.s3_state.selected_object < app.s3_state.object_scroll_offset {
                app.s3_state.object_scroll_offset = app.s3_state.selected_object;
            }
        }
    } else {
        app.s3_state.selected_row = app.s3_state.selected_row.saturating_sub(1);
        if app.s3_state.selected_row < app.s3_state.bucket_scroll_offset {
            app.s3_state.bucket_scroll_offset = app.s3_state.selected_row;
        }
    }
}

// ── Navigation — page down / up normal ───────────────────────────────────────

pub fn page_down_normal(app: &mut App) {
    if app.s3_state.current_bucket.is_none() {
        bucket_page_jump(app, true);
    } else {
        let total_rows = crate::ui::s3::calculate_total_object_rows(app);
        app.s3_state.selected_object = app
            .s3_state
            .selected_object
            .saturating_add(10)
            .min(total_rows.saturating_sub(1));
        let visible_rows = app.s3_state.object_visible_rows.get();
        if app.s3_state.selected_object >= app.s3_state.object_scroll_offset + visible_rows {
            app.s3_state.object_scroll_offset = app.s3_state.selected_object - visible_rows + 1;
        }
    }
}

pub fn page_up_normal(app: &mut App) {
    if app.s3_state.current_bucket.is_none() {
        bucket_page_jump(app, false);
    } else {
        app.s3_state.selected_object = app.s3_state.selected_object.saturating_sub(10);
        if app.s3_state.selected_object < app.s3_state.object_scroll_offset {
            app.s3_state.object_scroll_offset = app.s3_state.selected_object;
        }
    }
}

/// Jump the bucket list selection forward or backward by `page_size` top-level buckets,
/// ignoring expanded child rows. Keeps `selected_row` on a top-level bucket visual row.
fn bucket_page_jump(app: &mut App, forward: bool) {
    let filter = app.s3_state.buckets.filter.to_lowercase();
    let buckets: Vec<_> = app
        .s3_state
        .buckets
        .items
        .iter()
        .filter(|b| filter.is_empty() || b.name.to_lowercase().contains(&filter))
        .collect();

    if buckets.is_empty() {
        return;
    }

    // Compute visual start row for each top-level bucket
    let mut bucket_start_rows: Vec<usize> = Vec::with_capacity(buckets.len());
    let mut visual_row = 0usize;
    for bucket in &buckets {
        bucket_start_rows.push(visual_row);
        visual_row += 1;
        if app.s3_state.expanded_prefixes.contains(&bucket.name) {
            if app.s3_state.bucket_errors.contains_key(&bucket.name) {
                if let Some(err) = app.s3_state.bucket_errors.get(&bucket.name) {
                    let max_width = 120usize;
                    visual_row += if err.len() > max_width {
                        err.len().div_ceil(max_width)
                    } else {
                        1
                    };
                }
            } else if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                visual_row += count_visual_rows_of_objects(
                    preview,
                    &app.s3_state.expanded_prefixes,
                    &app.s3_state.prefix_preview,
                );
            }
        }
    }

    // Find which top-level bucket the current visual row is on
    let cur_row = app.s3_state.selected_row;
    let cur_idx = bucket_start_rows
        .iter()
        .rposition(|&r| r <= cur_row)
        .unwrap_or(0);

    let page = 10usize;
    let target_idx = if forward {
        (cur_idx + page).min(buckets.len() - 1)
    } else {
        cur_idx.saturating_sub(page)
    };

    let target_row = bucket_start_rows[target_idx];
    app.s3_state.selected_row = target_row;

    let visible = app.s3_state.bucket_visible_rows.get();
    if target_row >= app.s3_state.bucket_scroll_offset + visible {
        app.s3_state.bucket_scroll_offset = target_row.saturating_sub(visible - 1);
    } else if target_row < app.s3_state.bucket_scroll_offset {
        app.s3_state.bucket_scroll_offset = target_row;
    }
}

fn count_visual_rows_of_objects(
    objects: &[crate::app::S3Object],
    expanded_prefixes: &std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
) -> usize {
    let mut count = objects.len();
    for obj in objects {
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(nested) = prefix_preview.get(&obj.key) {
                count += count_visual_rows_of_objects(nested, expanded_prefixes, prefix_preview);
            } else {
                count += 1; // loading row
            }
        }
    }
    count
}

// ── Expand row ────────────────────────────────────────────────────────────────

/// Recursively find an object at a visual index in the objects tree.
fn find_object_at_visual_idx(
    obj: &crate::app::S3Object,
    visual_idx: &mut usize,
    target_idx: usize,
    expanded_prefixes: &std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    found_obj: &mut Option<crate::app::S3Object>,
) {
    if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
        if let Some(preview) = prefix_preview.get(&obj.key) {
            for nested_obj in preview {
                if *visual_idx == target_idx {
                    *found_obj = Some(nested_obj.clone());
                    return;
                }
                *visual_idx += 1;
                find_object_at_visual_idx(
                    nested_obj,
                    visual_idx,
                    target_idx,
                    expanded_prefixes,
                    prefix_preview,
                    found_obj,
                );
                if found_obj.is_some() {
                    return;
                }
            }
        } else {
            *visual_idx += 1; // loading row
        }
    }
}

/// Recursively expand/navigate a nested prefix in the bucket list.
#[allow(clippy::too_many_arguments)]
fn expand_nested(
    objects: &[crate::app::S3Object],
    row_idx: &mut usize,
    target_row: usize,
    expanded_prefixes: &mut std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    found: &mut bool,
    loading: &mut bool,
    selected_row: &mut usize,
) {
    for obj in objects {
        if *row_idx == target_row {
            if obj.is_prefix {
                if !expanded_prefixes.contains(&obj.key) {
                    expanded_prefixes.insert(obj.key.clone());
                    if !prefix_preview.contains_key(&obj.key) {
                        *loading = true;
                    }
                }
                if expanded_prefixes.contains(&obj.key) {
                    if let Some(preview) = prefix_preview.get(&obj.key) {
                        if !preview.is_empty() {
                            *selected_row = *row_idx + 1;
                        }
                    }
                }
            }
            *found = true;
            return;
        }
        *row_idx += 1;
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(nested) = prefix_preview.get(&obj.key) {
                expand_nested(
                    nested,
                    row_idx,
                    target_row,
                    expanded_prefixes,
                    prefix_preview,
                    found,
                    loading,
                    selected_row,
                );
                if *found {
                    return;
                }
            } else {
                *row_idx += 1; // loading row
            }
        }
    }
}

pub fn expand_row(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        // Objects view — expand prefix and move to first child
        let mut visual_idx = 0;
        let mut found_obj: Option<crate::app::S3Object> = None;

        for obj in &app.s3_state.objects {
            if visual_idx == app.s3_state.selected_object {
                found_obj = Some(obj.clone());
                break;
            }
            visual_idx += 1;
            find_object_at_visual_idx(
                obj,
                &mut visual_idx,
                app.s3_state.selected_object,
                &app.s3_state.expanded_prefixes,
                &app.s3_state.prefix_preview,
                &mut found_obj,
            );
            if found_obj.is_some() {
                break;
            }
        }

        if let Some(obj) = found_obj {
            if obj.is_prefix {
                if !app.s3_state.expanded_prefixes.contains(&obj.key) {
                    app.s3_state.expanded_prefixes.insert(obj.key.clone());
                    if !app.s3_state.prefix_preview.contains_key(&obj.key) {
                        app.s3_state.buckets.loading = true;
                    }
                }
                if app.s3_state.expanded_prefixes.contains(&obj.key) {
                    if let Some(preview) = app.s3_state.prefix_preview.get(&obj.key) {
                        if !preview.is_empty() {
                            app.s3_state.selected_object += 1;
                        }
                    }
                }
            }
        }
    } else {
        // Bucket list — expand bucket or nested prefix
        let mut row_idx = 0;
        let mut found = false;

        // Clone what we need to avoid borrow conflicts
        let buckets: Vec<_> = app.s3_state.buckets.items.clone();

        for bucket in &buckets {
            if row_idx == app.s3_state.selected_row {
                if !app.s3_state.expanded_prefixes.contains(&bucket.name) {
                    app.s3_state.expanded_prefixes.insert(bucket.name.clone());
                    if !app.s3_state.bucket_preview.contains_key(&bucket.name)
                        && !app.s3_state.bucket_errors.contains_key(&bucket.name)
                    {
                        app.s3_state.buckets.loading = true;
                    }
                }
                if app.s3_state.expanded_prefixes.contains(&bucket.name) {
                    if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                        if !preview.is_empty() {
                            app.s3_state.selected_row = row_idx + 1;
                            let visible = app.s3_state.bucket_visible_rows.get();
                            if app.s3_state.selected_row
                                >= app.s3_state.bucket_scroll_offset + visible
                            {
                                app.s3_state.bucket_scroll_offset =
                                    app.s3_state.selected_row.saturating_sub(visible - 1);
                            }
                        }
                    }
                }
                break;
            }
            row_idx += 1;

            if app.s3_state.bucket_errors.contains_key(&bucket.name)
                && app.s3_state.expanded_prefixes.contains(&bucket.name)
            {
                if let Some(err) = app.s3_state.bucket_errors.get(&bucket.name) {
                    let max_width = 120;
                    let error_rows = if err.len() > max_width {
                        err.len().div_ceil(max_width)
                    } else {
                        1
                    };
                    row_idx += error_rows;
                }
                continue;
            }

            if app.s3_state.expanded_prefixes.contains(&bucket.name) {
                if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name).cloned() {
                    expand_nested(
                        &preview,
                        &mut row_idx,
                        app.s3_state.selected_row,
                        &mut app.s3_state.expanded_prefixes,
                        &app.s3_state.prefix_preview,
                        &mut found,
                        &mut app.s3_state.buckets.loading,
                        &mut app.s3_state.selected_row,
                    );
                    if found || row_idx > app.s3_state.selected_row {
                        break;
                    }
                } else if row_idx > app.s3_state.selected_row {
                    break;
                }
            }

            if found {
                break;
            }
        }
    }
}

// ── Select item ───────────────────────────────────────────────────────────────

/// Recursively find an object at visual index for selection.
fn find_object_for_select(
    obj: &crate::app::S3Object,
    visual_idx: &mut usize,
    target_idx: usize,
    expanded_prefixes: &std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    found_obj: &mut Option<crate::app::S3Object>,
) {
    if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
        if let Some(preview) = prefix_preview.get(&obj.key) {
            for nested_obj in preview {
                if *visual_idx == target_idx {
                    *found_obj = Some(nested_obj.clone());
                    return;
                }
                *visual_idx += 1;
                find_object_for_select(
                    nested_obj,
                    visual_idx,
                    target_idx,
                    expanded_prefixes,
                    prefix_preview,
                    found_obj,
                );
                if found_obj.is_some() {
                    return;
                }
            }
        } else {
            *visual_idx += 1; // loading row
        }
    }
}

pub fn select_item(app: &mut App) {
    if app.s3_state.current_bucket.is_none() {
        // Bucket list — find the row and drill in
        let filtered_buckets: Vec<_> = app
            .s3_state
            .buckets
            .items
            .iter()
            .filter(|b| {
                app.s3_state.buckets.filter.is_empty()
                    || b.name
                        .to_lowercase()
                        .contains(&app.s3_state.buckets.filter.to_lowercase())
            })
            .cloned()
            .collect();

        let mut row_idx = 0;
        for bucket in &filtered_buckets {
            if row_idx == app.s3_state.selected_row {
                app.s3_state.current_bucket = Some(bucket.name.clone());
                app.s3_state.prefix_stack.clear();
                app.s3_state.buckets.loading = true;
                return;
            }
            row_idx += 1;

            if app.s3_state.bucket_errors.contains_key(&bucket.name)
                && app.s3_state.expanded_prefixes.contains(&bucket.name)
            {
                continue;
            }

            if app.s3_state.expanded_prefixes.contains(&bucket.name) {
                if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name).cloned() {
                    for obj in &preview {
                        if row_idx == app.s3_state.selected_row {
                            if obj.is_prefix {
                                app.s3_state.current_bucket = Some(bucket.name.clone());
                                app.s3_state.prefix_stack = vec![obj.key.clone()];
                                app.s3_state.buckets.loading = true;
                            }
                            return;
                        }
                        row_idx += 1;

                        if obj.is_prefix && app.s3_state.expanded_prefixes.contains(&obj.key) {
                            if let Some(nested) = app.s3_state.prefix_preview.get(&obj.key).cloned()
                            {
                                for nested_obj in &nested {
                                    if row_idx == app.s3_state.selected_row {
                                        if nested_obj.is_prefix {
                                            app.s3_state.current_bucket = Some(bucket.name.clone());
                                            app.s3_state.prefix_stack =
                                                vec![obj.key.clone(), nested_obj.key.clone()];
                                            app.s3_state.buckets.loading = true;
                                        }
                                        return;
                                    }
                                    row_idx += 1;
                                }
                            } else {
                                row_idx += 1;
                            }
                        }
                    }
                } else {
                    row_idx += 1;
                }
            }
        }
    } else {
        // Objects view — map visual index to object
        let mut visual_idx = 0;
        let mut found_obj: Option<crate::app::S3Object> = None;

        let objects = app.s3_state.objects.clone();
        for obj in &objects {
            if visual_idx == app.s3_state.selected_object {
                found_obj = Some(obj.clone());
                break;
            }
            visual_idx += 1;
            find_object_for_select(
                obj,
                &mut visual_idx,
                app.s3_state.selected_object,
                &app.s3_state.expanded_prefixes,
                &app.s3_state.prefix_preview,
                &mut found_obj,
            );
            if found_obj.is_some() {
                break;
            }
        }

        if let Some(obj) = found_obj {
            if obj.is_prefix {
                app.s3_state.prefix_stack.push(obj.key.clone());
                app.s3_state.buckets.loading = true;
            }
        }
    }
}

// ── Go to page ────────────────────────────────────────────────────────────────

pub fn go_to_page(app: &mut App, page: usize) {
    if app.s3_state.current_bucket.is_some() {
        let page_size = 50; // S3 objects use fixed page size
        let target = (page - 1) * page_size;
        let total_rows = crate::ui::s3::calculate_total_object_rows(app);
        app.s3_state.selected_object = target.min(total_rows.saturating_sub(1));
    } else {
        let page_size = app.s3_state.buckets.page_size.value();
        let target = (page - 1) * page_size;
        let total_rows = crate::ui::s3::calculate_total_bucket_rows(app);
        let max = total_rows.saturating_sub(1);
        app.s3_state.selected_row = target.min(max);
        app.s3_state.bucket_scroll_offset = target.min(total_rows.saturating_sub(page_size));
    }
}

// ── Prev pane (collapse / jump to parent) ────────────────────────────────────

pub fn prev_pane(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        prev_pane_objects(app);
    } else {
        prev_pane_buckets(app);
    }
}

fn prev_pane_objects(app: &mut App) {
    let mut visual_idx = 0;
    let mut found_obj: Option<crate::app::S3Object> = None;
    let mut parent_idx: Option<usize> = None;

    let objects = app.s3_state.objects.clone();
    find_with_parent(
        &objects,
        &mut visual_idx,
        app.s3_state.selected_object,
        &app.s3_state.expanded_prefixes,
        &app.s3_state.prefix_preview,
        &mut found_obj,
        &mut parent_idx,
        None,
    );

    if let Some(obj) = found_obj {
        if obj.is_prefix && app.s3_state.expanded_prefixes.contains(&obj.key) {
            app.s3_state.expanded_prefixes.remove(&obj.key);
            if let Some(parent) = parent_idx {
                app.s3_state.selected_object = parent;
            }
        } else if let Some(parent) = parent_idx {
            app.s3_state.selected_object = parent;
        }
    }

    adjust_object_scroll(app);
}

fn prev_pane_buckets(app: &mut App) {
    let mut row_idx = 0;
    let buckets = app.s3_state.buckets.items.clone();
    for bucket in &buckets {
        if row_idx == app.s3_state.selected_row {
            app.s3_state.expanded_prefixes.remove(&bucket.name);
            break;
        }
        row_idx += 1;
        if app.s3_state.expanded_prefixes.contains(&bucket.name) {
            if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name).cloned() {
                let mut found = false;
                let parent_row = row_idx - 1;
                collapse_nested(
                    &preview,
                    &mut row_idx,
                    app.s3_state.selected_row,
                    &mut app.s3_state.expanded_prefixes,
                    &app.s3_state.prefix_preview,
                    &mut found,
                    &mut app.s3_state.selected_row,
                    parent_row,
                );
                if found {
                    adjust_bucket_scroll(app);
                    return;
                }
            } else {
                row_idx += 1;
            }
        }
    }
    adjust_bucket_scroll(app);
}

// ── Collapse row ──────────────────────────────────────────────────────────────

pub fn collapse_row(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        return; // objects view handled by prev_pane
    }

    let filter = app.s3_state.buckets.filter.to_lowercase();
    let filtered: Vec<_> = app
        .s3_state
        .buckets
        .items
        .iter()
        .filter(|b| filter.is_empty() || b.name.to_lowercase().contains(&filter))
        .cloned()
        .collect();

    let mut row_idx = 0;
    for bucket in &filtered {
        if row_idx == app.s3_state.selected_row {
            app.s3_state.expanded_prefixes.remove(&bucket.name);
            break;
        }
        row_idx += 1;
        if app.s3_state.expanded_prefixes.contains(&bucket.name) {
            if app.s3_state.bucket_errors.contains_key(&bucket.name) {
                continue;
            }
            if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name).cloned() {
                let mut found = false;
                let parent_row = row_idx - 1;
                collapse_nested(
                    &preview,
                    &mut row_idx,
                    app.s3_state.selected_row,
                    &mut app.s3_state.expanded_prefixes,
                    &app.s3_state.prefix_preview,
                    &mut found,
                    &mut app.s3_state.selected_row,
                    parent_row,
                );
                if found {
                    adjust_bucket_scroll(app);
                    return;
                }
            } else {
                row_idx += 1;
            }
        }
    }
    adjust_bucket_scroll(app);
}

// ── Expand row ────────────────────────────────────────────────────────────────

pub fn expand_row_left(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        expand_objects(app);
    } else {
        expand_buckets(app);
    }
}

fn expand_buckets(app: &mut App) {
    let filter = app.s3_state.buckets.filter.to_lowercase();
    let filtered: Vec<_> = app
        .s3_state
        .buckets
        .items
        .iter()
        .filter(|b| filter.is_empty() || b.name.to_lowercase().contains(&filter))
        .cloned()
        .collect();

    let mut row_idx = 0;
    for bucket in &filtered {
        if row_idx == app.s3_state.selected_row {
            if app.s3_state.expanded_prefixes.contains(&bucket.name) {
                if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                    if !preview.is_empty() {
                        app.s3_state.selected_row = row_idx + 1;
                        adjust_bucket_scroll(app);
                    }
                }
            } else {
                app.s3_state.expanded_prefixes.insert(bucket.name.clone());
                app.s3_state.buckets.loading = true;
                if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name) {
                    if !preview.is_empty() {
                        app.s3_state.selected_row = row_idx + 1;
                        adjust_bucket_scroll(app);
                    }
                }
            }
            return;
        }
        row_idx += 1;

        if app.s3_state.expanded_prefixes.contains(&bucket.name) {
            if app.s3_state.bucket_errors.contains_key(&bucket.name) {
                continue;
            }
            if let Some(preview) = app.s3_state.bucket_preview.get(&bucket.name).cloned() {
                if let Some((loading, new_row)) = toggle_expand_at_row(
                    &preview,
                    &mut row_idx,
                    app.s3_state.selected_row,
                    &mut app.s3_state.expanded_prefixes,
                    &app.s3_state.prefix_preview,
                ) {
                    app.s3_state.selected_row = new_row;
                    if loading {
                        app.s3_state.buckets.loading = true;
                    }
                    adjust_bucket_scroll(app);
                    return;
                }
            }
        }
    }
}

fn expand_objects(app: &mut App) {
    let objects = app.s3_state.objects.clone();
    let mut row_idx = 0;
    if let Some((loading, new_row)) = toggle_expand_at_row(
        &objects,
        &mut row_idx,
        app.s3_state.selected_object,
        &mut app.s3_state.expanded_prefixes,
        &app.s3_state.prefix_preview,
    ) {
        app.s3_state.selected_object = new_row;
        if loading {
            app.s3_state.buckets.loading = true;
        }
    }
}

// ── Scroll offset helpers ─────────────────────────────────────────────────────

fn adjust_bucket_scroll(app: &mut App) {
    let visible = app.s3_state.bucket_visible_rows.get();
    let row = app.s3_state.selected_row;
    let offset = &mut app.s3_state.bucket_scroll_offset;
    if row < *offset {
        *offset = row;
    } else if row >= *offset + visible {
        *offset = row.saturating_sub(visible - 1);
    }
}

fn adjust_object_scroll(app: &mut App) {
    let visible = app.s3_state.object_visible_rows.get();
    let obj = app.s3_state.selected_object;
    let offset = &mut app.s3_state.object_scroll_offset;
    if obj < *offset {
        *offset = obj;
    } else if obj >= *offset + visible {
        *offset = obj.saturating_sub(visible - 1);
    }
}

// ── Tree traversal helpers ────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn find_with_parent(
    objects: &[crate::app::S3Object],
    visual_idx: &mut usize,
    target_idx: usize,
    expanded_prefixes: &std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    found_obj: &mut Option<crate::app::S3Object>,
    parent_idx: &mut Option<usize>,
    current_parent: Option<usize>,
) {
    for obj in objects {
        if *visual_idx == target_idx {
            *found_obj = Some(obj.clone());
            *parent_idx = current_parent;
            return;
        }
        let obj_idx = *visual_idx;
        *visual_idx += 1;
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(preview) = prefix_preview.get(&obj.key) {
                find_with_parent(
                    preview,
                    visual_idx,
                    target_idx,
                    expanded_prefixes,
                    prefix_preview,
                    found_obj,
                    parent_idx,
                    Some(obj_idx),
                );
                if found_obj.is_some() {
                    return;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collapse_nested(
    objects: &[crate::app::S3Object],
    row_idx: &mut usize,
    target_row: usize,
    expanded_prefixes: &mut std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
    found: &mut bool,
    selected_row: &mut usize,
    parent_row: usize,
) {
    for obj in objects {
        let current_row = *row_idx;
        if *row_idx == target_row {
            if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                expanded_prefixes.remove(&obj.key);
            }
            *selected_row = parent_row;
            *found = true;
            return;
        }
        *row_idx += 1;
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(nested) = prefix_preview.get(&obj.key).cloned() {
                collapse_nested(
                    &nested,
                    row_idx,
                    target_row,
                    expanded_prefixes,
                    prefix_preview,
                    found,
                    selected_row,
                    current_row,
                );
                if *found {
                    return;
                }
            } else {
                *row_idx += 1; // loading row
            }
        }
    }
}

/// Toggle expand/collapse a prefix at `target_row`. Returns `Some((needs_load, new_row))`.
fn toggle_expand_at_row(
    objects: &[crate::app::S3Object],
    row_idx: &mut usize,
    target_row: usize,
    expanded_prefixes: &mut std::collections::HashSet<String>,
    prefix_preview: &std::collections::HashMap<String, Vec<crate::app::S3Object>>,
) -> Option<(bool, usize)> {
    for obj in objects {
        if *row_idx == target_row {
            if obj.is_prefix {
                if expanded_prefixes.contains(&obj.key) {
                    expanded_prefixes.remove(&obj.key);
                    return Some((false, *row_idx));
                } else {
                    let needs_load = !prefix_preview.contains_key(&obj.key);
                    expanded_prefixes.insert(obj.key.clone());
                    return Some((needs_load, *row_idx + 1));
                }
            }
            return Some((false, *row_idx));
        }
        *row_idx += 1;
        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
            if let Some(nested) = prefix_preview.get(&obj.key).cloned() {
                if let Some(result) = toggle_expand_at_row(
                    &nested,
                    row_idx,
                    target_row,
                    expanded_prefixes,
                    prefix_preview,
                ) {
                    return Some(result);
                }
            }
        }
    }
    None
}
