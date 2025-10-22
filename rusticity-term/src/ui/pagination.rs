use ratatui::{layout::Rect, Frame};

use crate::common::{render_filter, render_pagination, FilterConfig};

pub struct PaginatedFilterConfig<'a> {
    pub filter_text: &'a str,
    pub is_filter_active: bool,
    pub selected_index: usize,
    pub total_items: usize,
    pub page_size: usize,
    pub area: Rect,
}

pub fn render_paginated_filter(frame: &mut Frame, config: PaginatedFilterConfig) {
    let total_pages = if config.total_items == 0 {
        1
    } else {
        config.total_items.div_ceil(config.page_size)
    };
    let current_page = if config.total_items == 0 {
        0
    } else {
        config.selected_index / config.page_size
    };

    let pagination = render_pagination(current_page, total_pages);

    render_filter(
        frame,
        FilterConfig {
            text: config.filter_text,
            placeholder: "Search",
            is_active: config.is_filter_active,
            right_content: vec![("", &pagination)],
            area: config.area,
        },
    );
}
