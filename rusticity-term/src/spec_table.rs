//! A table whose columns are a static spec of (header, width, cell): its rows,
//! filter, visible columns and sort order, plus the shared filter and paging
//! input and the shared rendering (filter bar above a bordered table). Used by
//! EventBridge, Elastic Load Balancing and the VPC subnet and security group
//! lists.
use crate::{
    app::App,
    async_read::ReadStatus,
    common::{render_pagination_text, InputFocus, SortDirection},
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    sort::SortOrder,
    table::{TableState, TableWindow},
    ui::{
        self,
        filter::{render_filter_bar, FilterConfig, FilterControl},
        loading::render_table,
        table::{Column, TableConfig},
    },
};
use ratatui::prelude::*;

/// A column: (id and header, cell). Its width fits the header and the rows
/// on screen; the last visible column takes the remaining space.
pub(crate) type Spec<T> = &'static [(&'static str, fn(&T) -> String)];

/// One table with its filter focus and visible columns.
pub(crate) struct Table<T: 'static> {
    pub(crate) rows: TableState<T>,
    pub(crate) visible: Vec<crate::common::ColumnId>,
    pub(crate) focus: InputFocus,
    pub(crate) sort: crate::sort::Choice,
    /// The order when no column is chosen. `None` keeps the read order and
    /// offers "Default order" in Sort by; a fixed order is marked there.
    default_sort: Option<SortOrder>,
    /// Text the filter also matches that is not a column, such as a Name tag.
    search: Option<fn(&T) -> String>,
    spec: Spec<T>,
}

impl<T: 'static> Table<T> {
    pub(crate) fn new(spec: Spec<T>) -> Self {
        Self {
            rows: TableState::new(),
            visible: spec.iter().map(|(id, ..)| *id).collect(),
            focus: InputFocus::Filter,
            sort: Default::default(),
            default_sort: None,
            search: None,
            spec,
        }
    }

    /// The filter also matches `text` of each row.
    pub(crate) fn searching(mut self, text: fn(&T) -> String) -> Self {
        self.search = Some(text);
        self
    }

    /// A table that is always ordered: `order` until another column is chosen.
    pub(crate) fn sorted_by(mut self, order: SortOrder) -> Self {
        self.default_sort = Some(order);
        self
    }

    /// Only these columns are shown at first; the rest are in preferences.
    pub(crate) fn showing(mut self, visible: &[crate::common::ColumnId]) -> Self {
        self.visible = self
            .spec
            .iter()
            .map(|(id, ..)| *id)
            .filter(|id| visible.contains(id))
            .collect();
        self
    }

    pub(crate) fn spec(&self) -> Spec<T> {
        self.spec
    }

    /// The order applied: the chosen column while it is visible, else the
    /// fixed default, else none (the read order).
    pub(crate) fn order(&self) -> Option<SortOrder> {
        self.sort.effective(&self.visible).or(self.default_sort)
    }

    /// Space on a "Sort by" row of this table.
    pub(crate) fn toggle_sort(&mut self, value: &'static str) {
        let Some(default) = self.default_sort else {
            self.sort.toggle(value, &self.visible);
            return;
        };
        let mut order = self.order().unwrap_or(default);
        order.toggle(value, default, &self.visible);
        self.sort = crate::sort::Choice {
            column: Some(order.column),
            direction: order.direction,
        };
    }

    /// Every column id, and the visible ones.
    pub(crate) fn ids(&self) -> (Vec<&'static str>, Vec<&'static str>) {
        (
            self.spec.iter().map(|(id, ..)| *id).collect(),
            self.visible.clone(),
        )
    }

    pub(crate) fn filtered(&self) -> Vec<&T> {
        let filter = self.rows.filter.to_lowercase();
        let mut rows: Vec<&T> = self
            .rows
            .items
            .iter()
            .filter(|row| {
                filter.is_empty()
                    || self
                        .spec
                        .iter()
                        .any(|(_, cell)| cell(row).to_lowercase().contains(&filter))
                    || self
                        .search
                        .is_some_and(|text| text(row).to_lowercase().contains(&filter))
            })
            .collect();
        let spec = self.spec;
        if let Some(order) = self.order() {
            crate::sort::apply(&mut rows, order, |row, id| {
                spec.iter()
                    .find(|(name, ..)| *name == id)
                    .map(|(_, cell)| cell(row))
                    .unwrap_or_default()
            });
        }
        rows
    }

    pub(crate) fn clear(&mut self) {
        self.rows.items.clear();
        self.rows.reset();
    }
}

/// A table of any row type, for preferences and the sort chooser.
pub(crate) trait AnyTable {
    /// Every column id, and the visible ones.
    fn columns(&self) -> (Vec<&'static str>, Vec<&'static str>);
    fn visible_mut(&mut self) -> &mut Vec<crate::common::ColumnId>;
    fn page_size(&self) -> crate::common::PageSize;
    fn set_page_size(&mut self, size: crate::common::PageSize);
    fn sort(&self) -> crate::sort::Choice;
    /// The fixed order of an always-ordered table, if it is one.
    fn default_sort(&self) -> Option<SortOrder>;
    /// Space on a "Sort by" row.
    fn toggle_sort(&mut self, value: &'static str);
    /// Back to the first row, as after a new order.
    fn reset(&mut self);
}

impl<T: 'static> AnyTable for Table<T> {
    fn columns(&self) -> (Vec<&'static str>, Vec<&'static str>) {
        self.ids()
    }
    fn visible_mut(&mut self) -> &mut Vec<crate::common::ColumnId> {
        &mut self.visible
    }
    fn page_size(&self) -> crate::common::PageSize {
        self.rows.page_size
    }
    fn set_page_size(&mut self, size: crate::common::PageSize) {
        self.rows.set_page_size(size)
    }
    fn sort(&self) -> crate::sort::Choice {
        self.sort
    }
    fn default_sort(&self) -> Option<SortOrder> {
        self.default_sort
    }
    fn toggle_sort(&mut self, value: &'static str) {
        Table::toggle_sort(self, value)
    }
    fn reset(&mut self) {
        self.rows.reset()
    }
}

const CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

/// The app state the shared table input needs besides the table.
pub(crate) struct Keys<'a> {
    pub(crate) mode: &'a mut Mode,
    pub(crate) page_input: &'a mut String,
    pub(crate) error_message: &'a mut Option<String>,
    pub(crate) error_scroll: &'a mut usize,
    pub(crate) clipboard: &'a crate::clipboard::ClipboardState,
    /// What `y` copies.
    pub(crate) selected_arn: Option<String>,
}

/// Filter focus, filter text, paging and row keys, as in every other table.
pub(crate) fn input<T: 'static>(table: &mut Table<T>, keys: Keys<'_>, action: &Action) -> bool {
    let Keys {
        mode,
        page_input,
        error_message,
        error_scroll,
        clipboard,
        selected_arn,
    } = keys;
    if *mode == Mode::FilterInput
        && matches!(action, Action::NextFilterFocus | Action::PrevFilterFocus)
    {
        table.focus = if *action == Action::NextFilterFocus {
            table.focus.next(&CONTROLS)
        } else {
            table.focus.prev(&CONTROLS)
        };
        page_input.clear();
        return true;
    }
    if *mode == Mode::FilterInput && *action == Action::DeleteWord {
        if table.focus == InputFocus::Filter {
            table.rows.filter_clear();
        } else {
            page_input.clear();
        }
        return true;
    }
    if *mode == Mode::Normal && *action == Action::StartFilter {
        table.focus = InputFocus::Filter;
        page_input.clear();
        *mode = Mode::FilterInput;
        return true;
    }
    let filtered_count = table.filtered().len();
    inventory::handle_action(
        InventoryInput {
            table: &mut table.rows,
            focus: &mut table.focus,
            mode,
            page_input,
            error_message,
            error_scroll,
            filtered_count,
            selected_arn,
            clipboard,
        },
        action,
    )
}

fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// A cell as shown: control characters blanked, empty as `-`.
pub(crate) fn value(text: &str) -> String {
    if text.is_empty() {
        "-".into()
    } else {
        clean(text)
    }
}

/// Texts of one table: its title, filter placeholder and empty-result line.
pub(crate) struct Texts<'a> {
    pub(crate) title: &'a str,
    pub(crate) placeholder: &'a str,
    pub(crate) no_items: &'a str,
}

/// The filter bar, then the table. `style` colors a cell by its text.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render<T: 'static>(
    frame: &mut Frame,
    app: &App,
    table: &Table<T>,
    texts: Texts<'_>,
    status: ReadStatus,
    error: Option<&str>,
    style: fn(&str) -> Style,
    area: Rect,
) {
    let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let prepared = table
        .rows
        .prepare_view(table.filtered(), TableWindow::SelectedPage);
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &table.rows.filter,
            placeholder: texts.placeholder,
            mode: app.mode,
            is_input_focused: table.focus == InputFocus::Filter,
            controls: vec![FilterControl {
                text: render_pagination_text(prepared.current_page, prepared.total_pages),
                is_focused: table.focus == InputFocus::Pagination,
            }],
            area: parts[0],
        },
    );
    let spec = table.spec();
    let columns: Vec<Box<dyn Column<T>>> = spec
        .iter()
        .filter(|(id, ..)| table.visible.contains(id))
        .map(|(name, cell)| {
            let cell = *cell;
            // Fit the header and the rows on screen, with no upper bound.
            Box::new(ui::table::OwnedColumn::new(
                name,
                name,
                u16::MAX,
                true,
                move |row: &T| {
                    let text = value(&cell(row));
                    let color = style(&text);
                    (text, color)
                },
            )) as Box<dyn Column<T>>
        })
        .collect();
    let sort_header = table
        .order()
        .filter(|o| table.visible.contains(&o.column))
        .map_or((String::new(), SortDirection::Asc), |o| {
            (o.column.to_string(), o.direction)
        });
    render_table(
        frame,
        TableConfig {
            items: prepared.items,
            selected_index: prepared.selected_index,
            expanded_index: prepared.expanded_index,
            columns: &columns,
            sort_column: &sort_header.0,
            sort_direction: sort_header.1,
            title: ui::format_title(&format!("{} ({})", texts.title, prepared.total_items)),
            area: parts[1],
            get_expanded_content: Some(Box::new(move |row: &T| {
                spec.iter()
                    .map(|(name, cell)| {
                        (format!("{name}: {}", value(&cell(row))), Style::default())
                    })
                    .collect()
            })),
            is_active: app.mode == Mode::Normal,
        },
        status,
        texts.title,
        error,
        ui::loading::EmptyState {
            no_items: texts.no_items,
            has_unfiltered_items: !table.rows.items.is_empty(),
        },
    );
}
