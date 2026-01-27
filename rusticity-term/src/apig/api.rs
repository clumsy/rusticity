use crate::common::translate_column;
use crate::common::{format_iso_timestamp, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone)]
pub struct RestApi {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_date: String,
    pub api_key_source: String,
    pub endpoint_configuration: String,
    pub protocol_type: String,
    pub disable_execute_api_endpoint: bool,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    Description,
    Id,
    Protocol,
    EndpointType,
    Created,
    SecurityPolicy,
    ApiStatus,
}

impl Column {
    const ID_NAME: &'static str = "column.apig.api.name";
    const ID_DESCRIPTION: &'static str = "column.apig.api.description";
    const ID_ID: &'static str = "column.apig.api.id";
    const ID_PROTOCOL: &'static str = "column.apig.api.protocol";
    const ID_ENDPOINT_TYPE: &'static str = "column.apig.api.endpoint_type";
    const ID_CREATED: &'static str = "column.apig.api.created";
    const ID_SECURITY_POLICY: &'static str = "column.apig.api.security_policy";
    const ID_API_STATUS: &'static str = "column.apig.api.api_status";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Name => Self::ID_NAME,
            Column::Description => Self::ID_DESCRIPTION,
            Column::Id => Self::ID_ID,
            Column::Protocol => Self::ID_PROTOCOL,
            Column::EndpointType => Self::ID_ENDPOINT_TYPE,
            Column::Created => Self::ID_CREATED,
            Column::SecurityPolicy => Self::ID_SECURITY_POLICY,
            Column::ApiStatus => Self::ID_API_STATUS,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::Description => "Description",
            Column::Id => "ID",
            Column::Protocol => "Protocol",
            Column::EndpointType => "API endpoint type",
            Column::Created => "Created",
            Column::SecurityPolicy => "Security policy",
            Column::ApiStatus => "API status",
        }
    }

    pub const fn all() -> [Column; 8] {
        [
            Column::Name,
            Column::Description,
            Column::Id,
            Column::Protocol,
            Column::EndpointType,
            Column::Created,
            Column::SecurityPolicy,
            Column::ApiStatus,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_DESCRIPTION => Some(Column::Description),
            Self::ID_ID => Some(Column::Id),
            Self::ID_PROTOCOL => Some(Column::Protocol),
            Self::ID_ENDPOINT_TYPE => Some(Column::EndpointType),
            Self::ID_CREATED => Some(Column::Created),
            Self::ID_SECURITY_POLICY => Some(Column::SecurityPolicy),
            Self::ID_API_STATUS => Some(Column::ApiStatus),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<RestApi> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Name => 30,
            Column::Description => 40,
            Column::Id => 15,
            Column::Protocol => 10,
            Column::EndpointType => 20,
            Column::Created => UTC_TIMESTAMP_WIDTH as usize,
            Column::SecurityPolicy => 18,
            Column::ApiStatus => 15,
        }) as u16
    }

    fn render(&self, item: &RestApi) -> (String, Style) {
        match self {
            Column::Name => (item.name.clone(), Style::default()),
            Column::Description => (item.description.clone(), Style::default()),
            Column::Id => (item.id.clone(), Style::default()),
            Column::Protocol => {
                // Capitalize protocol: Http -> HTTP, Websocket -> WEBSOCKET
                let protocol = match item.protocol_type.to_uppercase().as_str() {
                    "HTTP" => "HTTP",
                    "WEBSOCKET" => "WEBSOCKET",
                    "REST" => "REST",
                    _ => &item.protocol_type,
                };
                (protocol.to_string(), Style::default())
            }
            Column::EndpointType => {
                // Format endpoint type: REGIONAL -> Regional, EDGE -> Edge-optimized
                let endpoint = match item.endpoint_configuration.to_uppercase().as_str() {
                    "REGIONAL" => "Regional",
                    "EDGE" => "Edge-optimized",
                    "PRIVATE" => "Private",
                    _ => &item.endpoint_configuration,
                };
                (endpoint.to_string(), Style::default())
            }
            Column::Created => (format_iso_timestamp(&item.created_date), Style::default()),
            Column::SecurityPolicy => {
                // Security policy only applies to REST APIs with custom domains
                // Regional APIs don't have a security policy in the API itself
                let policy = if item.protocol_type.to_uppercase() == "REST" {
                    "-" // REST APIs show security policy on custom domain, not API
                } else {
                    "TLS 1.2"
                };
                (policy.to_string(), Style::default())
            }
            Column::ApiStatus => {
                // Show Available with green checkmark emoji
                if item.disable_execute_api_endpoint {
                    ("Disabled".to_string(), Style::default())
                } else {
                    (
                        "✅ Available".to_string(),
                        Style::default().fg(Color::Green),
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.apig.api."),
                "Column ID '{}' should start with 'column.apig.api.'",
                col.id()
            );
        }
    }

    #[test]
    fn test_all_columns_count() {
        assert_eq!(Column::all().len(), 8);
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(Column::from_id("column.apig.api.name"), Some(Column::Name));
    }

    #[test]
    fn test_protocol_capitalization() {
        use crate::ui::table::Column as TableColumn;

        let api = RestApi {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            created_date: "".to_string(),
            api_key_source: "".to_string(),
            endpoint_configuration: "".to_string(),
            protocol_type: "Http".to_string(),
            disable_execute_api_endpoint: false,
            status: "".to_string(),
        };

        let (text, _) = Column::Protocol.render(&api);
        assert_eq!(text, "HTTP");
    }

    #[test]
    fn test_endpoint_type_formatting() {
        use crate::ui::table::Column as TableColumn;

        let mut api = RestApi {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            created_date: "".to_string(),
            api_key_source: "".to_string(),
            endpoint_configuration: "REGIONAL".to_string(),
            protocol_type: "HTTP".to_string(),
            disable_execute_api_endpoint: false,
            status: "".to_string(),
        };

        let (text, _) = Column::EndpointType.render(&api);
        assert_eq!(text, "Regional");

        api.endpoint_configuration = "EDGE".to_string();
        let (text, _) = Column::EndpointType.render(&api);
        assert_eq!(text, "Edge-optimized");
    }

    #[test]
    fn test_api_status_available_green() {
        use crate::ui::table::Column as TableColumn;

        let api = RestApi {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            created_date: "".to_string(),
            api_key_source: "".to_string(),
            endpoint_configuration: "".to_string(),
            protocol_type: "HTTP".to_string(),
            disable_execute_api_endpoint: false,
            status: "".to_string(),
        };

        let (text, style) = Column::ApiStatus.render(&api);
        assert_eq!(text, "✅ Available");
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_api_status_disabled() {
        use crate::ui::table::Column as TableColumn;

        let api = RestApi {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            created_date: "".to_string(),
            api_key_source: "".to_string(),
            endpoint_configuration: "".to_string(),
            protocol_type: "HTTP".to_string(),
            disable_execute_api_endpoint: true,
            status: "".to_string(),
        };

        let (text, _) = Column::ApiStatus.render(&api);
        assert_eq!(text, "Disabled");
    }

    #[test]
    fn test_security_policy_rest_api() {
        use crate::ui::table::Column as TableColumn;

        let api = RestApi {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            created_date: "".to_string(),
            api_key_source: "".to_string(),
            endpoint_configuration: "".to_string(),
            protocol_type: "REST".to_string(),
            disable_execute_api_endpoint: false,
            status: "".to_string(),
        };

        let (text, _) = Column::SecurityPolicy.render(&api);
        assert_eq!(text, "-");
    }

    #[test]
    fn test_security_policy_http_api() {
        use crate::ui::table::Column as TableColumn;

        let api = RestApi {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            created_date: "".to_string(),
            api_key_source: "".to_string(),
            endpoint_configuration: "".to_string(),
            protocol_type: "HTTP".to_string(),
            disable_execute_api_endpoint: false,
            status: "".to_string(),
        };

        let (text, _) = Column::SecurityPolicy.render(&api);
        assert_eq!(text, "TLS 1.2");
    }
}
