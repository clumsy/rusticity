use crate::common::{translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::cfn::DetailTab;
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in [
        Column::Name,
        Column::StackId,
        Column::Status,
        Column::CreatedTime,
        Column::UpdatedTime,
        Column::DeletedTime,
        Column::DriftStatus,
        Column::LastDriftCheckTime,
        Column::StatusReason,
        Column::Description,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

pub fn console_url_stacks(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/cloudformation/home?region={}#/stacks",
        region, region
    )
}

pub fn console_url_stack_detail(region: &str, stack_name: &str, stack_id: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/cloudformation/home?region={}#/stacks/{}?filteringText=&filteringStatus=active&viewNested=true&stackId={}",
        region, region, stack_name, stack_id
    )
}

pub fn console_url_stack_detail_with_tab(region: &str, stack_id: &str, tab: &DetailTab) -> String {
    let tab_path = match tab {
        DetailTab::StackInfo => "stackinfo",
        DetailTab::Events => "events",
        DetailTab::Resources => "resources",
        DetailTab::Outputs => "outputs",
        DetailTab::Parameters => "parameters",
        DetailTab::Template => "template",
        DetailTab::ChangeSets => "changesets",
        DetailTab::GitSync => "gitsync",
    };
    let encoded_arn = urlencoding::encode(stack_id);
    format!(
        "https://{}.console.aws.amazon.com/cloudformation/home?region={}#/stacks/{}?filteringText=&filteringStatus=active&viewNested=true&stackId={}",
        region, region, tab_path, encoded_arn
    )
}

#[derive(Debug, Clone)]
pub struct Stack {
    pub name: String,
    pub stack_id: String,
    pub status: String,
    pub created_time: String,
    pub updated_time: String,
    pub deleted_time: String,
    pub drift_status: String,
    pub last_drift_check_time: String,
    pub status_reason: String,
    pub description: String,
    pub detailed_status: String,
    pub root_stack: String,
    pub parent_stack: String,
    pub termination_protection: bool,
    pub iam_role: String,
    pub tags: Vec<(String, String)>,
    pub stack_policy: String,
    pub rollback_monitoring_time: String,
    pub rollback_alarms: Vec<String>,
    pub notification_arns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    StackId,
    Status,
    CreatedTime,
    UpdatedTime,
    DeletedTime,
    DriftStatus,
    LastDriftCheckTime,
    StatusReason,
    Description,
}

impl Column {
    pub fn id(&self) -> &'static str {
        match self {
            Column::Name => "column.cfn.stack.name",
            Column::StackId => "column.cfn.stack.stack_id",
            Column::Status => "column.cfn.stack.status",
            Column::CreatedTime => "column.cfn.stack.created_time",
            Column::UpdatedTime => "column.cfn.stack.updated_time",
            Column::DeletedTime => "column.cfn.stack.deleted_time",
            Column::DriftStatus => "column.cfn.stack.drift_status",
            Column::LastDriftCheckTime => "column.cfn.stack.last_drift_check_time",
            Column::StatusReason => "column.cfn.stack.status_reason",
            Column::Description => "column.cfn.stack.description",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Stack Name",
            Column::StackId => "Stack ID",
            Column::Status => "Status",
            Column::CreatedTime => "Created Time",
            Column::UpdatedTime => "Updated Time",
            Column::DeletedTime => "Deleted Time",
            Column::DriftStatus => "Drift Status",
            Column::LastDriftCheckTime => "Last Drift Check Time",
            Column::StatusReason => "Status Reason",
            Column::Description => "Description",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.cfn.stack.name" => Some(Column::Name),
            "column.cfn.stack.stack_id" => Some(Column::StackId),
            "column.cfn.stack.status" => Some(Column::Status),
            "column.cfn.stack.created_time" => Some(Column::CreatedTime),
            "column.cfn.stack.updated_time" => Some(Column::UpdatedTime),
            "column.cfn.stack.deleted_time" => Some(Column::DeletedTime),
            "column.cfn.stack.drift_status" => Some(Column::DriftStatus),
            "column.cfn.stack.last_drift_check_time" => Some(Column::LastDriftCheckTime),
            "column.cfn.stack.status_reason" => Some(Column::StatusReason),
            "column.cfn.stack.description" => Some(Column::Description),
            _ => None,
        }
    }

    pub fn all() -> [Column; 10] {
        [
            Column::Name,
            Column::StackId,
            Column::Status,
            Column::CreatedTime,
            Column::UpdatedTime,
            Column::DeletedTime,
            Column::DriftStatus,
            Column::LastDriftCheckTime,
            Column::StatusReason,
            Column::Description,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn to_column(&self) -> Box<dyn TableColumn<&Stack>> {
        struct StackColumn {
            variant: Column,
        }

        impl TableColumn<&Stack> for StackColumn {
            fn name(&self) -> &str {
                Box::leak(self.variant.name().into_boxed_str())
            }

            fn width(&self) -> u16 {
                let translated = translate_column(self.variant.id(), self.variant.default_name());
                translated.len().max(match self.variant {
                    Column::Name => 30,
                    Column::StackId => 20,
                    Column::Status => 35,
                    Column::CreatedTime
                    | Column::UpdatedTime
                    | Column::DeletedTime
                    | Column::LastDriftCheckTime => UTC_TIMESTAMP_WIDTH as usize,
                    Column::DriftStatus => 20,
                    Column::StatusReason | Column::Description => 50,
                }) as u16
            }

            fn render(&self, item: &&Stack) -> (String, Style) {
                match self.variant {
                    Column::Name => (item.name.clone(), Style::default()),
                    Column::StackId => (item.stack_id.clone(), Style::default()),
                    Column::Status => {
                        let (formatted, color) = format_status(&item.status);
                        (formatted, Style::default().fg(color))
                    }
                    Column::CreatedTime => (item.created_time.clone(), Style::default()),
                    Column::UpdatedTime => (item.updated_time.clone(), Style::default()),
                    Column::DeletedTime => (item.deleted_time.clone(), Style::default()),
                    Column::DriftStatus => (item.drift_status.clone(), Style::default()),
                    Column::LastDriftCheckTime => {
                        (item.last_drift_check_time.clone(), Style::default())
                    }
                    Column::StatusReason => (item.status_reason.clone(), Style::default()),
                    Column::Description => (item.description.clone(), Style::default()),
                }
            }
        }

        Box::new(StackColumn { variant: *self })
    }
}

pub fn format_status(status: &str) -> (String, ratatui::style::Color) {
    let (emoji, color) = match status {
        "UPDATE_COMPLETE" | "CREATE_COMPLETE" | "DELETE_COMPLETE" | "IMPORT_COMPLETE" => {
            ("✅ ", ratatui::style::Color::Green)
        }
        "ROLLBACK_COMPLETE"
        | "UPDATE_ROLLBACK_COMPLETE"
        | "UPDATE_ROLLBACK_COMPLETE_CLEANUP_IN_PROGRESS"
        | "UPDATE_FAILED"
        | "CREATE_FAILED"
        | "DELETE_FAILED"
        | "ROLLBACK_FAILED"
        | "UPDATE_ROLLBACK_FAILED"
        | "IMPORT_ROLLBACK_FAILED"
        | "IMPORT_ROLLBACK_COMPLETE" => ("❌ ", ratatui::style::Color::Red),
        "UPDATE_IN_PROGRESS"
        | "UPDATE_COMPLETE_CLEANUP_IN_PROGRESS"
        | "DELETE_IN_PROGRESS"
        | "CREATE_IN_PROGRESS"
        | "ROLLBACK_IN_PROGRESS"
        | "UPDATE_ROLLBACK_IN_PROGRESS"
        | "REVIEW_IN_PROGRESS"
        | "IMPORT_IN_PROGRESS"
        | "IMPORT_ROLLBACK_IN_PROGRESS" => ("ℹ️  ", ratatui::style::Color::Blue),
        _ => ("", ratatui::style::Color::White),
    };

    (format!("{}{}", emoji, status), color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{CyclicEnum, SortDirection};
    use crate::ui::cfn::{DetailTab, State, StatusFilter};

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.table.items.len(), 0);
        assert_eq!(state.table.selected, 0);
        assert!(!state.table.loading);
        assert_eq!(state.table.filter, "");
        assert_eq!(state.status_filter, StatusFilter::All);
        assert!(!state.view_nested);
        assert_eq!(state.table.expanded_item, None);
        assert_eq!(state.current_stack, None);
        assert_eq!(state.detail_tab, DetailTab::StackInfo);
        assert_eq!(state.overview_scroll, 0);
        assert_eq!(state.sort_column, Column::CreatedTime);
        assert_eq!(state.sort_direction, SortDirection::Desc);
    }

    #[test]
    fn test_status_filter_names() {
        assert_eq!(StatusFilter::Active.name(), "Active");
        assert_eq!(StatusFilter::Complete.name(), "Complete");
        assert_eq!(StatusFilter::Failed.name(), "Failed");
        assert_eq!(StatusFilter::Deleted.name(), "Deleted");
        assert_eq!(StatusFilter::InProgress.name(), "In progress");
    }

    #[test]
    fn test_status_filter_next() {
        assert_eq!(StatusFilter::All.next(), StatusFilter::Active);
        assert_eq!(StatusFilter::Active.next(), StatusFilter::Complete);
        assert_eq!(StatusFilter::Complete.next(), StatusFilter::Failed);
        assert_eq!(StatusFilter::Failed.next(), StatusFilter::Deleted);
        assert_eq!(StatusFilter::Deleted.next(), StatusFilter::InProgress);
        assert_eq!(StatusFilter::InProgress.next(), StatusFilter::All);
    }

    #[test]
    fn test_status_filter_matches_active() {
        let filter = StatusFilter::Active;
        assert!(filter.matches("CREATE_IN_PROGRESS"));
        assert!(filter.matches("UPDATE_IN_PROGRESS"));
        assert!(!filter.matches("CREATE_COMPLETE"));
        assert!(!filter.matches("DELETE_COMPLETE"));
        assert!(!filter.matches("CREATE_FAILED"));
    }

    #[test]
    fn test_status_filter_matches_complete() {
        let filter = StatusFilter::Complete;
        assert!(filter.matches("CREATE_COMPLETE"));
        assert!(filter.matches("UPDATE_COMPLETE"));
        assert!(!filter.matches("DELETE_COMPLETE"));
        assert!(!filter.matches("CREATE_FAILED"));
        assert!(!filter.matches("CREATE_IN_PROGRESS"));
    }

    #[test]
    fn test_status_filter_matches_failed() {
        let filter = StatusFilter::Failed;
        assert!(filter.matches("CREATE_FAILED"));
        assert!(filter.matches("UPDATE_FAILED"));
        assert!(filter.matches("ROLLBACK_FAILED"));
        assert!(!filter.matches("CREATE_COMPLETE"));
        assert!(!filter.matches("DELETE_COMPLETE"));
    }

    #[test]
    fn test_status_filter_matches_deleted() {
        let filter = StatusFilter::Deleted;
        assert!(filter.matches("DELETE_COMPLETE"));
        assert!(filter.matches("DELETE_IN_PROGRESS"));
        assert!(filter.matches("DELETE_FAILED"));
        assert!(!filter.matches("CREATE_COMPLETE"));
        assert!(!filter.matches("UPDATE_FAILED"));
    }

    #[test]
    fn test_status_filter_matches_in_progress() {
        let filter = StatusFilter::InProgress;
        assert!(filter.matches("CREATE_IN_PROGRESS"));
        assert!(filter.matches("UPDATE_IN_PROGRESS"));
        assert!(filter.matches("DELETE_IN_PROGRESS"));
        assert!(!filter.matches("CREATE_COMPLETE"));
        assert!(!filter.matches("CREATE_FAILED"));
    }

    #[test]
    fn test_detail_tab_names() {
        assert_eq!(DetailTab::StackInfo.name(), "Stack info");
        assert_eq!(DetailTab::Events.name(), "Events");
        assert_eq!(DetailTab::Resources.name(), "Resources");
        assert_eq!(DetailTab::Outputs.name(), "Outputs");
        assert_eq!(DetailTab::Parameters.name(), "Parameters");
        assert_eq!(DetailTab::Template.name(), "Template");
        assert_eq!(DetailTab::ChangeSets.name(), "Change sets");
        assert_eq!(DetailTab::GitSync.name(), "Git sync");
    }

    #[test]
    fn test_detail_tab_next() {
        assert_eq!(DetailTab::StackInfo.next(), DetailTab::Events);
    }

    #[test]
    fn test_column_names() {
        assert_eq!(Column::Name.name(), "Stack Name");
        assert_eq!(Column::StackId.name(), "Stack ID");
        assert_eq!(Column::Status.name(), "Status");
        assert_eq!(Column::CreatedTime.name(), "Created Time");
        assert_eq!(Column::UpdatedTime.name(), "Updated Time");
        assert_eq!(Column::DeletedTime.name(), "Deleted Time");
        assert_eq!(Column::DriftStatus.name(), "Drift Status");
        assert_eq!(Column::LastDriftCheckTime.name(), "Last Drift Check Time");
        assert_eq!(Column::StatusReason.name(), "Status Reason");
        assert_eq!(Column::Description.name(), "Description");
    }

    #[test]
    fn test_column_all() {
        let columns = Column::ids();
        assert_eq!(columns.len(), 10);
        assert_eq!(columns[0], Column::Name.id());
        assert_eq!(columns[9], Column::Description.id());
    }

    #[test]
    fn test_format_status_complete_green() {
        let (formatted, color) = format_status("UPDATE_COMPLETE");
        assert_eq!(formatted, "✅ UPDATE_COMPLETE");
        assert_eq!(color, ratatui::style::Color::Green);

        let (formatted, color) = format_status("CREATE_COMPLETE");
        assert_eq!(formatted, "✅ CREATE_COMPLETE");
        assert_eq!(color, ratatui::style::Color::Green);

        let (formatted, color) = format_status("DELETE_COMPLETE");
        assert_eq!(formatted, "✅ DELETE_COMPLETE");
        assert_eq!(color, ratatui::style::Color::Green);
    }

    #[test]
    fn test_format_status_failed_red() {
        let (formatted, color) = format_status("UPDATE_FAILED");
        assert_eq!(formatted, "❌ UPDATE_FAILED");
        assert_eq!(color, ratatui::style::Color::Red);

        let (formatted, color) = format_status("CREATE_FAILED");
        assert_eq!(formatted, "❌ CREATE_FAILED");
        assert_eq!(color, ratatui::style::Color::Red);

        let (formatted, color) = format_status("DELETE_FAILED");
        assert_eq!(formatted, "❌ DELETE_FAILED");
        assert_eq!(color, ratatui::style::Color::Red);

        let (formatted, color) = format_status("ROLLBACK_FAILED");
        assert_eq!(formatted, "❌ ROLLBACK_FAILED");
        assert_eq!(color, ratatui::style::Color::Red);
    }

    #[test]
    fn test_format_status_rollback_red() {
        let (formatted, color) = format_status("ROLLBACK_COMPLETE");
        assert_eq!(formatted, "❌ ROLLBACK_COMPLETE");
        assert_eq!(color, ratatui::style::Color::Red);

        let (formatted, color) = format_status("UPDATE_ROLLBACK_COMPLETE");
        assert_eq!(formatted, "❌ UPDATE_ROLLBACK_COMPLETE");
        assert_eq!(color, ratatui::style::Color::Red);

        let (formatted, color) = format_status("UPDATE_ROLLBACK_COMPLETE_CLEANUP_IN_PROGRESS");
        assert_eq!(formatted, "❌ UPDATE_ROLLBACK_COMPLETE_CLEANUP_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Red);
    }

    #[test]
    fn test_format_status_in_progress_blue() {
        let (formatted, color) = format_status("UPDATE_IN_PROGRESS");
        assert_eq!(formatted, "ℹ️  UPDATE_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Blue);

        let (formatted, color) = format_status("CREATE_IN_PROGRESS");
        assert_eq!(formatted, "ℹ️  CREATE_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Blue);

        let (formatted, color) = format_status("DELETE_IN_PROGRESS");
        assert_eq!(formatted, "ℹ️  DELETE_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Blue);

        let (formatted, color) = format_status("UPDATE_COMPLETE_CLEANUP_IN_PROGRESS");
        assert_eq!(formatted, "ℹ️  UPDATE_COMPLETE_CLEANUP_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Blue);

        let (formatted, color) = format_status("ROLLBACK_IN_PROGRESS");
        assert_eq!(formatted, "ℹ️  ROLLBACK_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Blue);

        let (formatted, color) = format_status("UPDATE_ROLLBACK_IN_PROGRESS");
        assert_eq!(formatted, "ℹ️  UPDATE_ROLLBACK_IN_PROGRESS");
        assert_eq!(color, ratatui::style::Color::Blue);
    }

    #[test]
    fn test_format_status_unknown() {
        let (formatted, color) = format_status("UNKNOWN_STATUS");
        assert_eq!(formatted, "UNKNOWN_STATUS");
        assert_eq!(color, ratatui::style::Color::White);
    }

    #[test]
    fn test_format_status_emoji_spacing() {
        // Verify emojis have proper spacing to avoid overlay
        let (formatted, _) = format_status("CREATE_IN_PROGRESS");
        assert!(formatted.starts_with("ℹ️ ")); // One space after info emoji

        let (formatted, _) = format_status("CREATE_COMPLETE");
        assert!(formatted.starts_with("✅ ")); // One space after checkmark

        let (formatted, _) = format_status("CREATE_FAILED");
        assert!(formatted.starts_with("❌ ")); // One space after cross
    }

    #[test]
    fn test_all_aws_statuses_covered() {
        // Test all documented CloudFormation stack statuses
        let statuses = vec![
            "CREATE_IN_PROGRESS",
            "CREATE_FAILED",
            "CREATE_COMPLETE",
            "ROLLBACK_IN_PROGRESS",
            "ROLLBACK_FAILED",
            "ROLLBACK_COMPLETE",
            "DELETE_IN_PROGRESS",
            "DELETE_FAILED",
            "DELETE_COMPLETE",
            "UPDATE_IN_PROGRESS",
            "UPDATE_COMPLETE_CLEANUP_IN_PROGRESS",
            "UPDATE_COMPLETE",
            "UPDATE_FAILED",
            "UPDATE_ROLLBACK_IN_PROGRESS",
            "UPDATE_ROLLBACK_FAILED",
            "UPDATE_ROLLBACK_COMPLETE_CLEANUP_IN_PROGRESS",
            "UPDATE_ROLLBACK_COMPLETE",
            "REVIEW_IN_PROGRESS",
            "IMPORT_IN_PROGRESS",
            "IMPORT_COMPLETE",
            "IMPORT_ROLLBACK_IN_PROGRESS",
            "IMPORT_ROLLBACK_FAILED",
            "IMPORT_ROLLBACK_COMPLETE",
        ];

        for status in statuses {
            let (formatted, _) = format_status(status);
            // Ensure all statuses get formatted (no panics) and contain some text
            assert!(!formatted.is_empty());
            assert!(formatted.len() > 2); // More than just emoji
        }
    }

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.cfn.stack."),
                "Column ID '{}' should start with 'column.cfn.stack.'",
                col.id()
            );
        }
    }
}
