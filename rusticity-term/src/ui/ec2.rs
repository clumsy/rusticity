use crate::common::{CyclicEnum, SortDirection};
use crate::ec2::{Column, Instance};
use crate::keymap::Mode;
use crate::table::TableState;
use ratatui::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateFilter {
    AllStates,
    Running,
    Stopped,
    Terminated,
    Pending,
    ShuttingDown,
    Stopping,
}

impl StateFilter {
    pub fn name(&self) -> &'static str {
        match self {
            StateFilter::AllStates => "All states",
            StateFilter::Running => "Running",
            StateFilter::Stopped => "Stopped",
            StateFilter::Terminated => "Terminated",
            StateFilter::Pending => "Pending",
            StateFilter::ShuttingDown => "Shutting down",
            StateFilter::Stopping => "Stopping",
        }
    }

    pub fn matches(&self, state: &str) -> bool {
        match self {
            StateFilter::AllStates => true,
            StateFilter::Running => state == "running",
            StateFilter::Stopped => state == "stopped",
            StateFilter::Terminated => state == "terminated",
            StateFilter::Pending => state == "pending",
            StateFilter::ShuttingDown => state == "shutting-down",
            StateFilter::Stopping => state == "stopping",
        }
    }
}

impl CyclicEnum for StateFilter {
    const ALL: &'static [Self] = &[
        StateFilter::AllStates,
        StateFilter::Running,
        StateFilter::Stopped,
        StateFilter::Terminated,
        StateFilter::Pending,
        StateFilter::ShuttingDown,
        StateFilter::Stopping,
    ];

    fn next(&self) -> Self {
        match self {
            StateFilter::AllStates => StateFilter::Running,
            StateFilter::Running => StateFilter::Stopped,
            StateFilter::Stopped => StateFilter::Terminated,
            StateFilter::Terminated => StateFilter::Pending,
            StateFilter::Pending => StateFilter::ShuttingDown,
            StateFilter::ShuttingDown => StateFilter::Stopping,
            StateFilter::Stopping => StateFilter::AllStates,
        }
    }

    fn prev(&self) -> Self {
        match self {
            StateFilter::AllStates => StateFilter::Stopping,
            StateFilter::Running => StateFilter::AllStates,
            StateFilter::Stopped => StateFilter::Running,
            StateFilter::Terminated => StateFilter::Stopped,
            StateFilter::Pending => StateFilter::Terminated,
            StateFilter::ShuttingDown => StateFilter::Pending,
            StateFilter::Stopping => StateFilter::ShuttingDown,
        }
    }
}

impl Default for StateFilter {
    fn default() -> Self {
        StateFilter::AllStates
    }
}

pub struct State {
    pub table: TableState<Instance>,
    pub state_filter: StateFilter,
    pub sort_column: Column,
    pub sort_direction: SortDirection,
}

impl Default for State {
    fn default() -> Self {
        Self {
            table: TableState::default(),
            state_filter: StateFilter::default(),
            sort_column: Column::LaunchTime,
            sort_direction: SortDirection::Desc,
        }
    }
}

pub const FILTER_HINT: &str = "Find Instance by attribute or tag (case-sensitive)";

pub fn render_instances(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    visible_columns: &[&str],
    mode: Mode,
) {
    use crate::ui::filter::{render_filter_bar, FilterConfig, FilterControl};
    use crate::ui::table::render_table;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &state.table.filter,
            placeholder: FILTER_HINT,
            mode,
            is_input_focused: mode == Mode::FilterInput,
            controls: vec![FilterControl {
                text: state.state_filter.name().to_string(),
                is_focused: false,
            }],
            area: chunks[0],
        },
    );

    let columns: Vec<_> = visible_columns
        .iter()
        .filter_map(|id| Column::from_id(id).map(|c| c.to_column()))
        .collect();

    let filtered_items: Vec<_> = state
        .table
        .items
        .iter()
        .filter(|i| state.state_filter.matches(&i.state))
        .filter(|i| {
            if state.table.filter.is_empty() {
                return true;
            }
            i.name.contains(&state.table.filter)
                || i.instance_id.contains(&state.table.filter)
                || i.state.contains(&state.table.filter)
                || i.instance_type.contains(&state.table.filter)
                || i.availability_zone.contains(&state.table.filter)
                || i.security_groups.contains(&state.table.filter)
                || i.key_name.contains(&state.table.filter)
        })
        .cloned()
        .collect();

    let title = format!("Instances ({})", filtered_items.len());

    use crate::ui::table::TableConfig;
    render_table(
        frame,
        TableConfig {
            items: filtered_items.iter().collect(),
            selected_index: state.table.selected,
            expanded_index: None,
            columns: &columns,
            sort_column: "",
            sort_direction: state.sort_direction,
            title,
            area: chunks[1],
            get_expanded_content: None,
            is_active: false,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_filter_names() {
        assert_eq!(StateFilter::AllStates.name(), "All states");
        assert_eq!(StateFilter::Running.name(), "Running");
        assert_eq!(StateFilter::Stopped.name(), "Stopped");
        assert_eq!(StateFilter::Terminated.name(), "Terminated");
        assert_eq!(StateFilter::Pending.name(), "Pending");
        assert_eq!(StateFilter::ShuttingDown.name(), "Shutting down");
        assert_eq!(StateFilter::Stopping.name(), "Stopping");
    }

    #[test]
    fn test_state_filter_matches() {
        assert!(StateFilter::AllStates.matches("running"));
        assert!(StateFilter::AllStates.matches("stopped"));
        assert!(StateFilter::Running.matches("running"));
        assert!(!StateFilter::Running.matches("stopped"));
        assert!(StateFilter::Stopped.matches("stopped"));
        assert!(!StateFilter::Stopped.matches("running"));
    }

    #[test]
    fn test_state_filter_next() {
        assert_eq!(StateFilter::AllStates.next(), StateFilter::Running);
        assert_eq!(StateFilter::Running.next(), StateFilter::Stopped);
        assert_eq!(StateFilter::Stopping.next(), StateFilter::AllStates);
    }

    #[test]
    fn test_state_filter_prev() {
        assert_eq!(StateFilter::AllStates.prev(), StateFilter::Stopping);
        assert_eq!(StateFilter::Running.prev(), StateFilter::AllStates);
        assert_eq!(StateFilter::Stopped.prev(), StateFilter::Running);
    }

    #[test]
    fn test_state_filter_all_constant() {
        assert_eq!(StateFilter::ALL.len(), 7);
        assert_eq!(StateFilter::ALL[0], StateFilter::AllStates);
        assert_eq!(StateFilter::ALL[6], StateFilter::Stopping);
    }

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.table.items.len(), 0);
        assert_eq!(state.table.selected, 0);
        assert!(!state.table.loading);
        assert_eq!(state.table.filter, "");
        assert_eq!(state.state_filter, StateFilter::AllStates);
        assert_eq!(state.sort_column, Column::LaunchTime);
        assert_eq!(state.sort_direction, SortDirection::Desc);
    }

    #[test]
    fn test_state_filter_matches_all_states() {
        let filter = StateFilter::AllStates;
        assert!(filter.matches("running"));
        assert!(filter.matches("stopped"));
        assert!(filter.matches("terminated"));
        assert!(filter.matches("pending"));
        assert!(filter.matches("shutting-down"));
        assert!(filter.matches("stopping"));
    }

    #[test]
    fn test_state_filter_matches_specific_states() {
        assert!(StateFilter::Running.matches("running"));
        assert!(!StateFilter::Running.matches("stopped"));
        
        assert!(StateFilter::Stopped.matches("stopped"));
        assert!(!StateFilter::Stopped.matches("running"));
        
        assert!(StateFilter::Terminated.matches("terminated"));
        assert!(!StateFilter::Terminated.matches("running"));
        
        assert!(StateFilter::Pending.matches("pending"));
        assert!(!StateFilter::Pending.matches("running"));
        
        assert!(StateFilter::ShuttingDown.matches("shutting-down"));
        assert!(!StateFilter::ShuttingDown.matches("running"));
        
        assert!(StateFilter::Stopping.matches("stopping"));
        assert!(!StateFilter::Stopping.matches("running"));
    }

    #[test]
    fn test_state_filter_cycle() {
        let mut filter = StateFilter::AllStates;
        filter = filter.next();
        assert_eq!(filter, StateFilter::Running);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Stopped);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Terminated);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Pending);
        filter = filter.next();
        assert_eq!(filter, StateFilter::ShuttingDown);
        filter = filter.next();
        assert_eq!(filter, StateFilter::Stopping);
        filter = filter.next();
        assert_eq!(filter, StateFilter::AllStates);
    }
}
