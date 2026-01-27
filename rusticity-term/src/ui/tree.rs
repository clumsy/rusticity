use ratatui::prelude::*;
use ratatui::widgets::Cell;
use std::collections::{HashMap, HashSet};

pub const CURSOR_EXPANDED: &str = "▼";
pub const CURSOR_COLLAPSED: &str = "▶";

/// Trait for items that can be displayed in a tree structure
pub trait TreeItem {
    /// Unique identifier for this item
    fn id(&self) -> &str;

    /// Display name for this item
    fn display_name(&self) -> &str;

    /// Whether this item can have children (is expandable)
    fn is_expandable(&self) -> bool;

    /// Icon to display for this item
    fn icon(&self) -> &str {
        if self.is_expandable() {
            "📁"
        } else {
            "📄"
        }
    }
}

/// Generic tree renderer that handles hierarchical display with expand/collapse
pub struct TreeRenderer<'a, T: TreeItem> {
    /// All items at the current level
    pub items: &'a [T],

    /// Set of expanded item IDs
    pub expanded_ids: &'a HashSet<String>,

    /// Map of parent ID to children
    pub children_map: &'a HashMap<String, Vec<T>>,

    /// Currently selected row index
    pub selected_row: usize,

    /// Starting row index for this render
    pub start_row: usize,
}

impl<'a, T: TreeItem> TreeRenderer<'a, T> {
    pub fn new(
        items: &'a [T],
        expanded_ids: &'a HashSet<String>,
        children_map: &'a HashMap<String, Vec<T>>,
        selected_row: usize,
        start_row: usize,
    ) -> Self {
        Self {
            items,
            expanded_ids,
            children_map,
            selected_row,
            start_row,
        }
    }

    /// Render tree items recursively, returning rows with tree structure
    pub fn render<F>(&self, mut render_cell: F) -> Vec<(Vec<Cell<'a>>, Style)>
    where
        F: FnMut(&T, &str) -> Vec<Cell<'a>>,
    {
        let mut result = Vec::new();
        let mut current_row = self.start_row;

        self.render_recursive(
            self.items,
            &mut current_row,
            &mut result,
            "",
            &[],
            &mut render_cell,
        );

        result
    }

    fn render_recursive<F>(
        &self,
        items: &[T],
        current_row: &mut usize,
        result: &mut Vec<(Vec<Cell<'a>>, Style)>,
        _parent_id: &str,
        is_last: &[bool],
        render_cell: &mut F,
    ) where
        F: FnMut(&T, &str) -> Vec<Cell<'a>>,
    {
        for (idx, item) in items.iter().enumerate() {
            let is_last_item = idx == items.len() - 1;
            let is_expanded = self.expanded_ids.contains(item.id());

            // Build prefix with tree characters
            let mut prefix = String::new();
            for &last in is_last.iter() {
                prefix.push_str(if last { "  " } else { "│ " });
            }

            let tree_char = if is_last_item { "╰─" } else { "├─" };
            let expand_char = if item.is_expandable() {
                if is_expanded {
                    CURSOR_EXPANDED
                } else {
                    CURSOR_COLLAPSED
                }
            } else {
                ""
            };

            let icon = item.icon();
            let tree_prefix = format!("{}{}{} {} ", prefix, tree_char, expand_char, icon);

            // Determine style based on selection
            let style = if *current_row == self.selected_row {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            // Render cells for this item
            let cells = render_cell(item, &tree_prefix);
            result.push((cells, style));
            *current_row += 1;

            // Recursively render children if expanded
            if item.is_expandable() && is_expanded {
                if let Some(children) = self.children_map.get(item.id()) {
                    let mut new_is_last = is_last.to_vec();
                    new_is_last.push(is_last_item);
                    self.render_recursive(
                        children,
                        current_row,
                        result,
                        item.id(),
                        &new_is_last,
                        render_cell,
                    );
                }
            }
        }
    }

    /// Count total visible rows including expanded children
    pub fn count_visible_rows(
        items: &[T],
        expanded_ids: &HashSet<String>,
        children_map: &HashMap<String, Vec<T>>,
    ) -> usize {
        let mut count = 0;
        for item in items {
            count += 1;
            if item.is_expandable() && expanded_ids.contains(item.id()) {
                if let Some(children) = children_map.get(item.id()) {
                    count += Self::count_visible_rows(children, expanded_ids, children_map);
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestItem {
        id: String,
        name: String,
        is_folder: bool,
    }

    impl TreeItem for TestItem {
        fn id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn is_expandable(&self) -> bool {
            self.is_folder
        }
    }

    #[test]
    fn test_count_visible_rows_no_expansion() {
        let items = vec![
            TestItem {
                id: "1".to_string(),
                name: "folder1".to_string(),
                is_folder: true,
            },
            TestItem {
                id: "2".to_string(),
                name: "file1".to_string(),
                is_folder: false,
            },
        ];

        let expanded = HashSet::new();
        let children = HashMap::new();

        let count = TreeRenderer::count_visible_rows(&items, &expanded, &children);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_visible_rows_with_expansion() {
        let items = vec![TestItem {
            id: "1".to_string(),
            name: "folder1".to_string(),
            is_folder: true,
        }];

        let mut expanded = HashSet::new();
        expanded.insert("1".to_string());

        let mut children = HashMap::new();
        children.insert(
            "1".to_string(),
            vec![
                TestItem {
                    id: "1/a".to_string(),
                    name: "file_a".to_string(),
                    is_folder: false,
                },
                TestItem {
                    id: "1/b".to_string(),
                    name: "file_b".to_string(),
                    is_folder: false,
                },
            ],
        );

        let count = TreeRenderer::count_visible_rows(&items, &expanded, &children);
        assert_eq!(count, 3); // 1 folder + 2 children
    }

    #[test]
    fn test_count_visible_rows_nested_expansion() {
        let items = vec![TestItem {
            id: "1".to_string(),
            name: "folder1".to_string(),
            is_folder: true,
        }];

        let mut expanded = HashSet::new();
        expanded.insert("1".to_string());
        expanded.insert("1/a".to_string());

        let mut children = HashMap::new();
        children.insert(
            "1".to_string(),
            vec![TestItem {
                id: "1/a".to_string(),
                name: "folder_a".to_string(),
                is_folder: true,
            }],
        );
        children.insert(
            "1/a".to_string(),
            vec![TestItem {
                id: "1/a/x".to_string(),
                name: "file_x".to_string(),
                is_folder: false,
            }],
        );

        let count = TreeRenderer::count_visible_rows(&items, &expanded, &children);
        assert_eq!(count, 3); // 1 folder + 1 subfolder + 1 file
    }

    #[test]
    fn test_tree_item_default_icons() {
        let folder = TestItem {
            id: "1".to_string(),
            name: "folder".to_string(),
            is_folder: true,
        };
        let file = TestItem {
            id: "2".to_string(),
            name: "file".to_string(),
            is_folder: false,
        };

        assert_eq!(folder.icon(), "📁");
        assert_eq!(file.icon(), "📄");
    }
}
