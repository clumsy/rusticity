use crate::app::App;
use crate::common::{
    filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection,
};
use crate::keymap::Mode;
use crate::rds::instance::Column;
use crate::table::{TableState, TableWindow};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{expanded_from_columns, render_table, TableConfig};
use crate::ui::{format_title, titled_block};
use ratatui::{prelude::*, widgets::Paragraph};
use rusticity_core::rds::RdsInstance;

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct State {
    pub instances: TableState<RdsInstance>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            instances: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
        }
    }
}

pub fn filtered_instances(app: &App) -> Vec<&RdsInstance> {
    let mut instances = filter_by_fields(
        &app.rds_state.instances.items,
        &app.rds_state.instances.filter,
        |instance| {
            [
                &instance.identifier,
                &instance.engine,
                &instance.engine_version,
                &instance.status,
                &instance.instance_class,
                &instance.region,
                &instance.availability_zone,
                &instance.endpoint,
                &instance.arn,
                &instance.cluster_identifier,
                &instance.vpc,
                &instance.subnet_group,
            ]
        },
    );
    instances.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    instances
}

pub fn render_instances(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.rds_state;
    let view = state
        .instances
        .prepare_view(filtered_instances(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &state.instances.filter,
            placeholder: "Search by DB identifier, engine, status, class, AZ, or endpoint",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            is_pagination_focused: state.input_focus == InputFocus::Pagination,
        },
    );
    if state.instances.loading && state.instances.items.is_empty() {
        frame.render_widget(
            Paragraph::new("Loading databases...")
                .block(titled_block("Databases"))
                .alignment(Alignment::Center),
            chunks[1],
        );
        return;
    }
    let columns: Vec<_> = state
        .visible_column_ids
        .iter()
        .filter_map(|id| Column::from_id(id))
        .map(Column::into_table_column)
        .collect();
    render_table(
        frame,
        TableConfig {
            items: view.items,
            selected_index: view.selected_index,
            expanded_index: view.expanded_index,
            columns: &columns,
            sort_column: &Column::Identifier.name(),
            sort_direction: SortDirection::Asc,
            title: format_title(&format!("Databases ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|instance| {
                expanded_from_columns(&columns, instance)
            })),
            is_active: app.mode == Mode::Normal,
        },
    );
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    fn instance(identifier: &str, engine: &str, status: &str) -> RdsInstance {
        RdsInstance {
            identifier: identifier.to_string(),
            engine: engine.to_string(),
            status: status.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn searches_multiple_fields_case_insensitively_and_keeps_identifier_sort() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.rds_state.instances.items = vec![
            instance("db-zeta", "postgres", "available"),
            instance("db-alpha", "mysql", "available"),
        ];

        // Engine match returns only the mysql row.
        app.rds_state.instances.filter = "MYSQL".to_string();
        let f = filtered_instances(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].identifier, "db-alpha");

        // Shared "available" status match → both, sorted by identifier ascending.
        app.rds_state.instances.filter = "available".to_string();
        let f = filtered_instances(&app);
        assert_eq!(
            f.iter().map(|i| i.identifier.as_str()).collect::<Vec<_>>(),
            ["db-alpha", "db-zeta"]
        );
    }

    #[test]
    fn empty_filter_returns_all_and_whitespace_is_significant() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.rds_state.instances.items = vec![instance("db-1", "postgres", "available")];

        app.rds_state.instances.filter = String::new();
        assert_eq!(filtered_instances(&app).len(), 1);

        // No trimming: a padded query must not match the tight field value.
        app.rds_state.instances.filter = " db-1 ".to_string();
        assert!(filtered_instances(&app).is_empty());
    }

    #[test]
    fn unicode_case_folding_matches() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.rds_state.instances.items = vec![instance("café-db", "postgres", "available")];
        app.rds_state.instances.filter = "CAFÉ".to_string();
        assert_eq!(filtered_instances(&app).len(), 1);
    }
}
