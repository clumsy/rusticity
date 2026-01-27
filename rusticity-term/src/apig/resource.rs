use crate::ui::table::Column as TableColumn;
use crate::ui::tree::TreeItem;
use ratatui::prelude::*;
pub use rusticity_core::apig::Resource;

impl TreeItem for Resource {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn is_expandable(&self) -> bool {
        !self.methods.is_empty()
    }

    fn icon(&self) -> &str {
        ""
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Path,
    Id,
    Arn,
}

impl Column {
    const ID_PATH: &'static str = "path";
    const ID_ID: &'static str = "id";
    const ID_ARN: &'static str = "arn";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Path => Self::ID_PATH,
            Column::Id => Self::ID_ID,
            Column::Arn => Self::ID_ARN,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_PATH => Some(Column::Path),
            Self::ID_ID => Some(Column::Id),
            Self::ID_ARN => Some(Column::Arn),
            _ => None,
        }
    }

    pub const fn all() -> [Column; 3] {
        [Column::Path, Column::Id, Column::Arn]
    }

    pub fn ids() -> Vec<&'static str> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<Resource> for Column {
    fn name(&self) -> &str {
        match self {
            Column::Path => "Resource",
            Column::Id => "ID",
            Column::Arn => "ARN",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Column::Path => 40,
            Column::Id => 20,
            Column::Arn => 60,
        }
    }

    fn render(&self, item: &Resource) -> (String, Style) {
        let text = match self {
            Column::Path => item.path.clone(),
            Column::Id => item.id.clone(),
            Column::Arn => item.arn.clone(),
        };
        (text, Style::default())
    }
}
