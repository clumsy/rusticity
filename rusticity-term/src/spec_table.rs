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
    /// Lines an expanded row shows after its columns.
    details: Option<fn(&T) -> Vec<String>>,
    /// The filter text is a query the service runs, not a local filter.
    query: bool,
    /// A menu in the filter line, between the text and the pages.
    dropdown: Option<Dropdown>,
    /// The rows are the first part of a larger result.
    pub(crate) partial: bool,
    spec: Spec<T>,
}

/// A menu of fixed choices in the filter line. Tab reaches it, arrows or
/// Space move the highlight while the list is open, and the owner acts on
/// Enter with `dropdown_choice`.
pub(crate) struct Dropdown {
    label: &'static str,
    options: &'static [&'static str],
    highlighted: usize,
}

/// The focus of a table dropdown.
pub(crate) const DROPDOWN: InputFocus = InputFocus::Dropdown("SpecTableDropdown");

impl<T: 'static> Table<T> {
    pub(crate) fn new(spec: Spec<T>) -> Self {
        Self {
            rows: TableState::new(),
            visible: spec.iter().map(|(id, ..)| *id).collect(),
            focus: InputFocus::Filter,
            sort: Default::default(),
            default_sort: None,
            search: None,
            details: None,
            query: false,
            dropdown: None,
            partial: false,
            spec,
        }
    }

    /// The filter text is a query for the service: rows are never matched
    /// against it here.
    pub(crate) fn querying(mut self) -> Self {
        self.query = true;
        self
    }

    /// A `label` menu of `options` in the filter line.
    pub(crate) fn with_dropdown(
        mut self,
        label: &'static str,
        options: &'static [&'static str],
    ) -> Self {
        self.dropdown = Some(Dropdown {
            label,
            options,
            highlighted: 0,
        });
        self
    }

    /// The highlighted option while the dropdown has focus.
    pub(crate) fn dropdown_choice(&self) -> Option<usize> {
        let dropdown = self.dropdown.as_ref()?;
        (self.focus == DROPDOWN).then_some(dropdown.highlighted)
    }

    /// The filter line controls, in Tab order.
    fn controls(&self) -> Vec<InputFocus> {
        let mut controls = vec![InputFocus::Filter];
        if self.dropdown.is_some() {
            controls.push(DROPDOWN);
        }
        controls.push(InputFocus::Pagination);
        controls
    }

    /// An expanded row also shows `lines` of each row, after its columns.
    pub(crate) fn detailing(mut self, lines: fn(&T) -> Vec<String>) -> Self {
        self.details = Some(lines);
        self
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
                self.query
                    || filter.is_empty()
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
        self.partial = false;
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
        let controls = table.controls();
        table.focus = if *action == Action::NextFilterFocus {
            table.focus.next(&controls)
        } else {
            table.focus.prev(&controls)
        };
        page_input.clear();
        return true;
    }
    if *mode == Mode::FilterInput && table.focus == DROPDOWN {
        if let Some(dropdown) = table.dropdown.as_mut() {
            let len = dropdown.options.len().max(1);
            match action {
                Action::NextItem | Action::ToggleFilterCheckbox => {
                    dropdown.highlighted = (dropdown.highlighted + 1) % len;
                    return true;
                }
                Action::PrevItem => {
                    dropdown.highlighted = (dropdown.highlighted + len - 1) % len;
                    return true;
                }
                // The list takes no text.
                Action::FilterInput(_) | Action::FilterBackspace | Action::DeleteWord => {
                    return true
                }
                _ => {}
            }
        }
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
    let pagination = render_pagination_text(prepared.current_page, prepared.total_pages);
    let mut controls = Vec::new();
    if let Some(dropdown) = &table.dropdown {
        controls.push(FilterControl {
            text: dropdown.label.to_string(),
            is_focused: table.focus == DROPDOWN,
        });
    }
    controls.push(FilterControl {
        text: pagination.clone(),
        is_focused: table.focus == InputFocus::Pagination,
    });
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &table.rows.filter,
            placeholder: texts.placeholder,
            mode: app.mode,
            is_input_focused: table.focus == InputFocus::Filter,
            controls,
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
    let details = table.details;
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
            title: ui::format_title(&format!(
                "{} ({}{})",
                texts.title,
                prepared.total_items,
                if table.partial { "+" } else { "" }
            )),
            area: parts[1],
            get_expanded_content: Some(Box::new(move |row: &T| {
                let columns = spec
                    .iter()
                    .map(|(name, cell)| format!("{name}: {}", value(&cell(row))));
                let extra = details.map(|lines| lines(row)).unwrap_or_default();
                columns
                    .chain(extra.iter().map(|line| clean(line)))
                    .map(|line| (line, Style::default()))
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
    // The open list, drawn last so it lies over the table.
    if let Some(dropdown) = &table.dropdown {
        if app.mode == Mode::FilterInput && table.focus == DROPDOWN {
            // Right of the list: the separator, the pages and the border.
            let after = 3 + pagination.chars().count() as u16 + 2;
            crate::common::render_dropdown(
                frame,
                dropdown.options,
                dropdown.highlighted,
                parts[0],
                after,
            );
        }
    }
}
