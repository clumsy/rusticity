//! Shared sort chooser: one column and one direction per table, picked in the
//! Preferences panel ("Sort by", "Sort direction"). The order is applied in
//! the table's filtered-rows function, so rendering, selection, paging,
//! expansion, copy and print all see the same order.
use crate::common::{ColumnId, SortDirection};
use crate::preferences::Preferences;
use std::cmp::Ordering;

pub(crate) mod tables;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SortOrder {
    pub column: ColumnId,
    pub direction: SortDirection,
}

impl SortOrder {
    pub const fn new(column: ColumnId, direction: SortDirection) -> Self {
        Self { column, direction }
    }

    /// The order actually applied: the chosen column while it is visible,
    /// else the table default, so a hidden column never orders the rows.
    pub fn effective(self, default: SortOrder, visible: &[&str]) -> SortOrder {
        if visible.contains(&self.column) {
            self
        } else {
            default
        }
    }

    /// Space on a "Sort by" row: the column in effect flips direction, any
    /// other column becomes the order, ascending. `visible` decides which
    /// column is in effect, so a hidden choice starts over from the default.
    pub fn toggle(&mut self, value: &'static str, default: SortOrder, visible: &[&str]) {
        let current = self.effective(default, visible);
        *self = if current.column == value {
            SortOrder::new(value, flip(current.direction))
        } else {
            SortOrder::new(value, SortDirection::Asc)
        };
    }
}

fn flip(direction: SortDirection) -> SortDirection {
    match direction {
        SortDirection::Asc => SortDirection::Desc,
        SortDirection::Desc => SortDirection::Asc,
    }
}

/// The value of the "Default order" row: keep the order the table had
/// before any choice, which is each view's own order.
pub(crate) const DEFAULT_VALUE: &str = "default";

/// A user choice for a table whose default order is its own: no column
/// means that order is kept. Used by every table except the five lists
/// that have an explicit `SortOrder` default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Choice {
    pub column: Option<ColumnId>,
    pub direction: SortDirection,
}

impl Choice {
    /// The order to apply, if any: a chosen column that is still visible.
    pub fn effective(self, visible: &[&str]) -> Option<SortOrder> {
        self.column
            .filter(|c| visible.contains(c))
            .map(|column| SortOrder::new(column, self.direction))
    }

    /// Space on a "Sort by" row: "Default order" clears the choice, the
    /// column in effect flips direction, any other column starts ascending.
    pub fn toggle(&mut self, value: &'static str, visible: &[&str]) {
        *self = if value == DEFAULT_VALUE {
            Choice::default()
        } else if self.effective(visible).is_some_and(|o| o.column == value) {
            Choice {
                column: Some(value),
                direction: flip(self.direction),
            }
        } else {
            Choice {
                column: Some(value),
                direction: SortDirection::Asc,
            }
        };
    }

    /// Sort `rows` when a column is in effect; otherwise leave them as they are.
    pub fn apply<T>(
        self,
        rows: &mut Vec<&T>,
        visible: &[&str],
        cell: impl Fn(&T, ColumnId) -> String,
    ) {
        if let Some(order) = self.effective(visible) {
            apply(rows, order, cell);
        }
    }

    /// Header name and direction to show: the chosen column, else `fallback`.
    pub fn header(
        self,
        visible: &[&str],
        name: impl Fn(ColumnId) -> Option<String>,
        fallback: (&str, SortDirection),
    ) -> (String, SortDirection) {
        self.effective(visible)
            .and_then(|o| name(o.column).map(|n| (n, o.direction)))
            .unwrap_or((fallback.0.to_string(), fallback.1))
    }
}

/// "Sort by" with a "Default order" row first, then the visible columns.
/// The column in effect shows its direction.
pub(crate) fn with_choice(
    prefs: Preferences,
    columns: &[(ColumnId, &'static str)],
    choice: Choice,
    visible: &[&str],
) -> Preferences {
    let mut choices = vec![(DEFAULT_VALUE, "Default order")];
    choices.extend_from_slice(columns);
    let selected = match choice.effective(visible) {
        Some(order) => (order.column, Some(order.direction)),
        None => (DEFAULT_VALUE, None),
    };
    prefs.sort_group(&choices, selected)
}

/// The text a table column shows for `row`; empty for an unknown column.
pub fn cell<T, C: crate::ui::table::Column<T>>(column: Option<C>, row: &T) -> String {
    column.map(|c| c.render(row).0).unwrap_or_default()
}

/// The header name of a table column; `None` for an unknown column.
pub fn name<T, C: crate::ui::table::Column<T>>(column: Option<C>) -> Option<String> {
    column.map(|c| c.name().to_string())
}

/// "Sort by" (the visible columns), marking the order in effect.
pub(crate) fn with_sort(
    prefs: Preferences,
    choices: &[(ColumnId, &'static str)],
    order: SortOrder,
) -> Preferences {
    prefs.sort_group(choices, (order.column, Some(order.direction)))
}

fn blank(text: &str) -> bool {
    let text = text.trim();
    text.is_empty() || text == "-"
}

/// Case-insensitive compare that orders digit runs by value, so `db-2`
/// sorts before `db-10` and `20 GiB` before `100 GiB`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(c);
                        it.next();
                    }
                    digits
                };
                let (x, y) = (take(&mut a), take(&mut b));
                let (xt, yt) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let order = xt.len().cmp(&yt.len()).then_with(|| xt.cmp(yt));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

/// Stable sort by the text the column shows. Rows with no value ("-" or
/// empty) stay last in both directions.
pub fn apply<T>(rows: &mut Vec<&T>, order: SortOrder, cell: impl Fn(&T, ColumnId) -> String) {
    let mut keyed: Vec<(String, &T)> = rows
        .drain(..)
        .map(|row| (cell(row, order.column), row))
        .collect();
    keyed.sort_by(|(a, _), (b, _)| match (blank(a), blank(b)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            let order_ab = natural_cmp(a, b);
            match order.direction {
                SortDirection::Asc => order_ab,
                SortDirection::Desc => order_ab.reverse(),
            }
        }
    });
    rows.extend(keyed.into_iter().map(|(_, row)| row));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_compares_numbers_by_value_and_ignores_case() {
        let mut names = vec!["db-10", "DB-2", "db-1", "alpha", "20 GiB", "100 GiB"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            ["20 GiB", "100 GiB", "alpha", "db-1", "DB-2", "db-10"]
        );
        assert_eq!(natural_cmp("a007", "a7"), Ordering::Equal);
    }

    #[test]
    fn blanks_stay_last_in_both_directions_and_ties_keep_input_order() {
        let rows = ["b", "-", "a", "", "a"];
        for (direction, expected) in [
            (SortDirection::Asc, ["a", "a", "b", "-", ""]),
            (SortDirection::Desc, ["b", "a", "a", "-", ""]),
        ] {
            let mut sorted: Vec<&&str> = rows.iter().collect();
            apply(&mut sorted, SortOrder::new("x", direction), |r, _| {
                r.to_string()
            });
            let got: Vec<&str> = sorted.into_iter().copied().collect();
            assert_eq!(got, expected, "{direction:?}");
        }
    }

    #[test]
    fn a_hidden_column_falls_back_to_the_default_order() {
        let default = SortOrder::new("name", SortDirection::Asc);
        let chosen = SortOrder::new("size", SortDirection::Desc);
        assert_eq!(chosen.effective(default, &["name", "size"]), chosen);
        assert_eq!(chosen.effective(default, &["name"]), default);
    }

    #[test]
    fn space_picks_a_column_ascending_then_flips_it() {
        let default = SortOrder::new("name", SortDirection::Desc);
        let visible = ["name", "size"];
        let mut order = default;
        order.toggle("size", default, &visible);
        assert_eq!(order, SortOrder::new("size", SortDirection::Asc));
        order.toggle("size", default, &visible);
        assert_eq!(order, SortOrder::new("size", SortDirection::Desc));
        // The default column in effect flips too.
        let mut order = default;
        order.toggle("name", default, &visible);
        assert_eq!(order, SortOrder::new("name", SortDirection::Asc));
        // A hidden choice starts over from the default.
        let mut order = SortOrder::new("gone", SortDirection::Desc);
        order.toggle("size", default, &visible);
        assert_eq!(order, SortOrder::new("size", SortDirection::Asc));
    }

    #[test]
    fn choice_default_row_clears_and_columns_flip() {
        let visible = ["name"];
        let mut choice = Choice::default();
        choice.toggle("name", &visible);
        assert_eq!(
            choice.effective(&visible),
            Some(SortOrder::new("name", SortDirection::Asc))
        );
        choice.toggle("name", &visible);
        assert_eq!(
            choice.effective(&visible),
            Some(SortOrder::new("name", SortDirection::Desc))
        );
        choice.toggle(DEFAULT_VALUE, &visible);
        assert_eq!(choice.effective(&visible), None);
    }
}
