//! Shared sort chooser: one column and one direction per table, picked in the
//! Preferences panel ("Sort by", "Sort direction"). The order is applied in
//! the table's filtered-rows function, so rendering, selection, paging,
//! expansion, copy and print all see the same order.
use crate::common::{ColumnId, SortDirection};
use crate::preferences::Preferences;
use std::cmp::Ordering;

pub(crate) const COLUMN_GROUP: &str = "sort-column";
pub(crate) const DIRECTION_GROUP: &str = "sort-direction";

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

    /// Apply a Preferences radio choice. Returns true when it was a sort row.
    pub fn choose(&mut self, group: &str, value: &'static str) -> bool {
        match group {
            COLUMN_GROUP => self.column = value,
            DIRECTION_GROUP => {
                self.direction = if value == "desc" {
                    SortDirection::Desc
                } else {
                    SortDirection::Asc
                }
            }
            _ => return false,
        }
        true
    }
}

/// Add "Sort by" (the visible columns) and "Sort direction" to a table's
/// preferences, marking the order in effect.
pub(crate) fn with_sort(
    prefs: Preferences,
    choices: &[(ColumnId, &'static str)],
    order: SortOrder,
) -> Preferences {
    let direction = match order.direction {
        SortDirection::Asc => "asc",
        SortDirection::Desc => "desc",
    };
    prefs
        .radio_group(COLUMN_GROUP, "Sort by", choices, order.column)
        .radio_group(
            DIRECTION_GROUP,
            "Sort direction",
            &[("asc", "Ascending"), ("desc", "Descending")],
            direction,
        )
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
    fn radio_choices_set_column_and_direction_only() {
        let mut order = SortOrder::new("name", SortDirection::Asc);
        assert!(order.choose(COLUMN_GROUP, "size"));
        assert!(order.choose(DIRECTION_GROUP, "desc"));
        assert!(!order.choose("alarm-view", "cards"));
        assert_eq!(order, SortOrder::new("size", SortDirection::Desc));
    }
}
