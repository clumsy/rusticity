use crate::ui::table::Column as TableColumn;
use crate::ui::tree::TreeItem;
use ratatui::prelude::*;

#[derive(Debug, Clone)]
pub struct Route {
    pub route_id: String,
    pub route_key: String,
    pub target: String,
    pub authorization_type: String,
    pub api_key_required: bool,
    pub display_name: String,
    pub arn: String,
}

impl TreeItem for Route {
    fn id(&self) -> &str {
        &self.route_key
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn is_expandable(&self) -> bool {
        // Virtual parent nodes (empty target) are expandable
        // They're created specifically to hold children
        self.target.is_empty()
    }

    fn icon(&self) -> &str {
        if self.route_key == "$default"
            || self.route_key == "$connect"
            || self.route_key == "$disconnect"
        {
            "🔌" // WebSocket special routes
        } else {
            "" // No icon, use tree expand/collapse indicators
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    RouteKey,
    RouteId,
    Arn,
    AuthorizationType,
    Target,
}

impl Column {
    const ID_ROUTE_KEY: &'static str = "route_key";
    const ID_ROUTE_ID: &'static str = "route_id";
    const ID_ARN: &'static str = "arn";
    const ID_AUTHORIZATION_TYPE: &'static str = "authorization_type";
    const ID_TARGET: &'static str = "target";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::RouteKey => Self::ID_ROUTE_KEY,
            Column::RouteId => Self::ID_ROUTE_ID,
            Column::Arn => Self::ID_ARN,
            Column::AuthorizationType => Self::ID_AUTHORIZATION_TYPE,
            Column::Target => Self::ID_TARGET,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_ROUTE_KEY => Some(Column::RouteKey),
            Self::ID_ROUTE_ID => Some(Column::RouteId),
            Self::ID_ARN => Some(Column::Arn),
            Self::ID_AUTHORIZATION_TYPE => Some(Column::AuthorizationType),
            Self::ID_TARGET => Some(Column::Target),
            _ => None,
        }
    }

    pub const fn all() -> [Column; 5] {
        [
            Column::RouteKey,
            Column::RouteId,
            Column::Arn,
            Column::AuthorizationType,
            Column::Target,
        ]
    }
}

impl TableColumn<Route> for Column {
    fn name(&self) -> &str {
        match self {
            Column::RouteKey => "Route",
            Column::RouteId => "ID",
            Column::Arn => "ARN",
            Column::AuthorizationType => "Authorization",
            Column::Target => "Integration",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Column::RouteKey => 30,
            Column::RouteId => 15,
            Column::Arn => 50,
            Column::AuthorizationType => 15,
            Column::Target => 30,
        }
    }

    fn render(&self, item: &Route) -> (String, Style) {
        let text = match self {
            Column::RouteKey => item.route_key.clone(),
            Column::RouteId => item.route_id.clone(),
            Column::Arn => item.arn.clone(),
            Column::AuthorizationType => item.authorization_type.clone(),
            Column::Target => item.target.clone(),
        };
        (text, Style::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_tree_item_id() {
        let route = Route {
            route_id: "abc123".to_string(),
            route_key: "/api/users".to_string(),
            target: "integration1".to_string(),
            authorization_type: "NONE".to_string(),
            api_key_required: false,
            display_name: String::new(),
            arn: String::new(),
        };
        assert_eq!(route.id(), "/api/users");
    }

    #[test]
    fn test_route_tree_item_display_name() {
        let route = Route {
            route_id: "abc123".to_string(),
            route_key: "/api/users/{id}".to_string(),
            target: "integration1".to_string(),
            authorization_type: "NONE".to_string(),
            api_key_required: false,
            display_name: "/api/users/{id}".to_string(),
            arn: String::new(),
        };
        assert_eq!(route.display_name(), "/api/users/{id}");
    }

    #[test]
    fn test_route_is_expandable() {
        // Virtual parent (empty target) is expandable
        let virtual_parent = Route {
            route_id: "virtual_/v1".to_string(),
            route_key: "/v1".to_string(),
            target: String::new(),
            authorization_type: String::new(),
            api_key_required: false,
            display_name: String::new(),
            arn: String::new(),
        };
        assert!(virtual_parent.is_expandable());

        // Real routes (with target) are not expandable
        let real_route = Route {
            route_id: "1".to_string(),
            route_key: "/api/users".to_string(),
            target: "int1".to_string(),
            authorization_type: "NONE".to_string(),
            api_key_required: false,
            display_name: String::new(),
            arn: String::new(),
        };
        assert!(!real_route.is_expandable());
    }

    #[test]
    fn test_route_icons() {
        let websocket_default = Route {
            route_id: "3".to_string(),
            route_key: "$default".to_string(),
            target: "int3".to_string(),
            authorization_type: "NONE".to_string(),
            api_key_required: false,
            display_name: String::new(),
            arn: String::new(),
        };
        assert_eq!(websocket_default.icon(), "🔌");

        let websocket_connect = Route {
            route_id: "4".to_string(),
            route_key: "$connect".to_string(),
            target: "int4".to_string(),
            authorization_type: "NONE".to_string(),
            api_key_required: false,
            display_name: String::new(),
            arn: String::new(),
        };
        assert_eq!(websocket_connect.icon(), "🔌");

        // Regular routes have no icon
        let regular_route = Route {
            route_id: "1".to_string(),
            route_key: "/api/users".to_string(),
            target: "int1".to_string(),
            authorization_type: "NONE".to_string(),
            api_key_required: false,
            display_name: String::new(),
            arn: String::new(),
        };
        assert_eq!(regular_route.icon(), "");
    }

    #[test]
    fn test_build_route_hierarchy_flat() {
        use crate::ui::apig::build_route_hierarchy;

        let routes = vec![
            Route {
                route_id: "1".to_string(),
                route_key: "/users".to_string(),
                target: "int1".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
            Route {
                route_id: "2".to_string(),
                route_key: "/health".to_string(),
                target: "int2".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
        ];

        let (root, children) = build_route_hierarchy(routes);
        assert_eq!(root.len(), 2);
        assert!(children.is_empty());
    }

    #[test]
    fn test_build_route_hierarchy_nested() {
        use crate::ui::apig::build_route_hierarchy;

        let routes = vec![
            Route {
                route_id: "1".to_string(),
                route_key: "/api/users".to_string(),
                target: "int1".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
            Route {
                route_id: "2".to_string(),
                route_key: "/api/users/{id}".to_string(),
                target: "int2".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
        ];

        let (root, children) = build_route_hierarchy(routes);
        // Virtual /api parent created at root
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].route_key, "/api");

        // /api has /api/users as child
        assert!(children.contains_key("/api"));
        assert_eq!(children.get("/api").unwrap().len(), 1);
        assert_eq!(children.get("/api").unwrap()[0].route_key, "/api/users");

        // /api/users has /api/users/{id} as child
        assert!(children.contains_key("/api/users"));
        assert_eq!(children.get("/api/users").unwrap().len(), 1);
    }

    #[test]
    fn test_build_route_hierarchy_websocket() {
        use crate::ui::apig::build_route_hierarchy;

        let routes = vec![
            Route {
                route_id: "1".to_string(),
                route_key: "$default".to_string(),
                target: "int1".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
            Route {
                route_id: "2".to_string(),
                route_key: "$connect".to_string(),
                target: "int2".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
            Route {
                route_id: "3".to_string(),
                route_key: "/users".to_string(),
                target: "int3".to_string(),
                authorization_type: "NONE".to_string(),
                api_key_required: false,
                display_name: String::new(),
                arn: String::new(),
            },
        ];

        let (root, children) = build_route_hierarchy(routes);
        assert_eq!(root.len(), 3);
        assert!(children.is_empty());
    }
}
