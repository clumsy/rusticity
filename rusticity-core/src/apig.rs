use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Route {
    pub route_id: String,
    pub route_key: String,
    pub target: String,
    pub authorization_type: String,
    pub api_key_required: bool,
    pub arn: String,
}

#[derive(Clone, Debug)]
pub struct Resource {
    pub id: String,
    pub path: String,
    pub parent_id: Option<String>,
    pub methods: Vec<String>,
    pub display_name: String,
    pub arn: String,
}

impl Resource {
    pub fn methods_display(&self) -> String {
        if self.methods.is_empty() {
            String::new()
        } else {
            self.methods.join(", ")
        }
    }
}

pub struct ApiGatewayClient {
    config: AwsConfig,
}

impl ApiGatewayClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_rest_apis(&self) -> Result<Vec<RestApi>> {
        let mut all_apis = Vec::new();

        // List REST APIs (v1)
        let v1_client = self.config.apigateway_client().await;
        let mut position: Option<String> = None;

        loop {
            let mut request = v1_client.get_rest_apis();
            if let Some(pos) = position {
                request = request.position(pos);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for api in items {
                    all_apis.push(RestApi {
                        id: api.id.unwrap_or_default(),
                        name: api.name.unwrap_or_default(),
                        description: api.description.unwrap_or_default(),
                        created_date: api
                            .created_date
                            .map(|dt| {
                                dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                        api_key_source: api
                            .api_key_source
                            .map(|s| format!("{:?}", s))
                            .unwrap_or_default(),
                        endpoint_configuration: api
                            .endpoint_configuration
                            .and_then(|ec| ec.types)
                            .map(|types| {
                                types
                                    .iter()
                                    .map(|t| format!("{:?}", t))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default(),
                        protocol_type: "REST".to_string(),
                        disable_execute_api_endpoint: api.disable_execute_api_endpoint,
                        status: "AVAILABLE".to_string(),
                    });
                }
            }

            position = response.position;
            if position.is_none() {
                break;
            }
        }

        // List HTTP/WebSocket APIs (v2)
        let v2_client = self.config.apigatewayv2_client().await;
        let mut next_token: Option<String> = None;

        loop {
            let mut request = v2_client.get_apis();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for api in items {
                    all_apis.push(RestApi {
                        id: api.api_id.unwrap_or_default(),
                        name: api.name.unwrap_or_default(),
                        description: api.description.unwrap_or_default(),
                        created_date: api
                            .created_date
                            .map(|dt| {
                                dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                        api_key_source: "N/A".to_string(),
                        endpoint_configuration: api
                            .api_endpoint
                            .map(|ep| {
                                if ep.contains("execute-api") {
                                    "REGIONAL".to_string()
                                } else {
                                    "CUSTOM".to_string()
                                }
                            })
                            .unwrap_or_default(),
                        protocol_type: api
                            .protocol_type
                            .map(|p| format!("{:?}", p))
                            .unwrap_or_default(),
                        disable_execute_api_endpoint: api
                            .disable_execute_api_endpoint
                            .unwrap_or(false),
                        status: "AVAILABLE".to_string(),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(all_apis)
    }

    pub async fn list_routes(&self, api_id: &str) -> Result<Vec<Route>> {
        let v2_client = self.config.apigatewayv2_client().await;
        let mut routes = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = v2_client.get_routes().api_id(api_id);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for route in items {
                    let route_id = route.route_id.unwrap_or_default();
                    let arn = format!(
                        "arn:aws:apigateway:{}::/apis/{}/routes/{}",
                        self.config.region, api_id, route_id
                    );
                    routes.push(Route {
                        route_id,
                        route_key: route.route_key.unwrap_or_default(),
                        target: route.target.unwrap_or_default(),
                        authorization_type: route
                            .authorization_type
                            .map(|t| format!("{:?}", t))
                            .unwrap_or_else(|| "NONE".to_string()),
                        api_key_required: route.api_key_required.unwrap_or(false),
                        arn,
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(routes)
    }

    pub async fn list_resources(&self, api_id: &str) -> Result<Vec<Resource>> {
        let v1_client = self.config.apigateway_client().await;
        let region = &self.config.region;
        let mut resources = Vec::new();
        let mut position: Option<String> = None;

        loop {
            let mut request = v1_client.get_resources().rest_api_id(api_id);
            if let Some(pos) = position {
                request = request.position(pos);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for resource in items {
                    let methods = resource
                        .resource_methods
                        .map(|m| m.keys().map(|k| k.to_string()).collect())
                        .unwrap_or_default();

                    let path = resource.path.unwrap_or_default();
                    let display = if path == "/" {
                        "/".to_string()
                    } else {
                        path.rsplit('/')
                            .next()
                            .map(|s| format!("/{}", s))
                            .unwrap_or(path.clone())
                    };

                    let arn = format!(
                        "arn:aws:apigateway:{}::/restapis/{}/resources/{}",
                        region,
                        api_id,
                        resource.id.as_ref().unwrap_or(&String::new())
                    );

                    resources.push(Resource {
                        id: resource.id.unwrap_or_default(),
                        path,
                        parent_id: resource.parent_id,
                        methods,
                        display_name: display,
                        arn,
                    });
                }
            }

            position = response.position;
            if position.is_none() {
                break;
            }
        }

        Ok(resources)
    }
}
