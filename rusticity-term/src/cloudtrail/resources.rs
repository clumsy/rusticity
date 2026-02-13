use crate::common::ColumnId;
use crate::ui::table::Column as TableColumn;
use ratatui::style::Style;

#[derive(Debug, Clone)]
pub struct EventResource {
    pub resource_type: String,
    pub resource_name: String,
    pub timeline: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventResourceColumn {
    ResourceType,
    ResourceName,
    Timeline,
}

impl EventResourceColumn {
    const ID_RESOURCE_TYPE: &'static str = "column.cloudtrail.resource.resource_type";
    const ID_RESOURCE_NAME: &'static str = "column.cloudtrail.resource.resource_name";
    const ID_TIMELINE: &'static str = "column.cloudtrail.resource.timeline";

    pub fn id(&self) -> ColumnId {
        match self {
            Self::ResourceType => Self::ID_RESOURCE_TYPE,
            Self::ResourceName => Self::ID_RESOURCE_NAME,
            Self::Timeline => Self::ID_TIMELINE,
        }
    }

    pub fn from_id(id: &ColumnId) -> Option<Self> {
        match *id {
            Self::ID_RESOURCE_TYPE => Some(Self::ResourceType),
            Self::ID_RESOURCE_NAME => Some(Self::ResourceName),
            Self::ID_TIMELINE => Some(Self::Timeline),
            _ => None,
        }
    }

    pub fn ids() -> Vec<ColumnId> {
        vec![
            Self::ID_RESOURCE_TYPE,
            Self::ID_RESOURCE_NAME,
            Self::ID_TIMELINE,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ResourceType => "Resource type",
            Self::ResourceName => "Resource name",
            Self::Timeline => "AWS Config resource timeline",
        }
    }

    pub fn width(&self) -> u16 {
        match self {
            Self::ResourceType => 30,
            Self::ResourceName => 50,
            Self::Timeline => 30,
        }
    }
}

impl TableColumn<EventResource> for EventResourceColumn {
    fn name(&self) -> &str {
        self.name()
    }

    fn width(&self) -> u16 {
        self.width()
    }

    fn render(&self, resource: &EventResource) -> (String, Style) {
        let value = match self {
            Self::ResourceType => resource.resource_type.clone(),
            Self::ResourceName => resource.resource_name.clone(),
            Self::Timeline => resource.timeline.clone(),
        };
        (value, Style::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for id in EventResourceColumn::ids() {
            assert!(id.starts_with("column.cloudtrail.resource."));
        }
    }

    #[test]
    fn test_all_columns_count() {
        assert_eq!(EventResourceColumn::ids().len(), 3);
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(
            EventResourceColumn::from_id(&EventResourceColumn::ID_RESOURCE_TYPE),
            Some(EventResourceColumn::ResourceType)
        );
        assert_eq!(
            EventResourceColumn::from_id(&EventResourceColumn::ID_RESOURCE_NAME),
            Some(EventResourceColumn::ResourceName)
        );
        assert_eq!(
            EventResourceColumn::from_id(&EventResourceColumn::ID_TIMELINE),
            Some(EventResourceColumn::Timeline)
        );
        assert_eq!(EventResourceColumn::from_id(&"invalid"), None);
    }
}
