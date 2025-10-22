use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct LambdaFunction {
    pub name: String,
    pub arn: String,
    pub application: Option<String>,
    pub description: String,
    pub package_type: String,
    pub runtime: String,
    pub architecture: String,
    pub code_size: i64,
    pub code_sha256: String,
    pub memory_mb: i32,
    pub timeout_seconds: i32,
    pub last_modified: String,
    pub layers: Vec<LambdaLayer>,
}

#[derive(Clone, Debug)]
pub struct LambdaLayer {
    pub arn: String,
    pub code_size: i64,
}

#[derive(Clone, Debug)]
pub struct LambdaVersion {
    pub version: String,
    pub aliases: String,
    pub description: String,
    pub last_modified: String,
    pub architecture: String,
}

#[derive(Clone, Debug)]
pub struct LambdaAlias {
    pub name: String,
    pub versions: String,
    pub description: String,
}

pub struct LambdaClient {
    config: AwsConfig,
}

impl LambdaClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_functions(&self) -> Result<Vec<LambdaFunction>> {
        let client = self.config.lambda_client().await;

        let mut functions = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = client.list_functions().max_items(100);
            if let Some(marker) = next_marker {
                request = request.marker(marker);
            }

            let response = request.send().await?;

            if let Some(funcs) = response.functions {
                for func in funcs {
                    // AWS returns last_modified in format: "2024-10-31T12:30:45.000+0000"
                    let last_modified = func
                        .last_modified
                        .as_deref()
                        .map(|s| {
                            // Try parsing with timezone
                            if let Ok(dt) =
                                chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3f%z")
                            {
                                dt.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                                dt.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            } else {
                                s.to_string()
                            }
                        })
                        .unwrap_or_default();

                    let function_name = func.function_name.unwrap_or_default();

                    // Extract application name from function name pattern
                    // e.g., "storefront-studio-beta-api" -> "storefront-studio-beta"
                    let application = function_name
                        .rsplit_once('-')
                        .map(|(prefix, _)| prefix.to_string());

                    let layers = func
                        .layers
                        .unwrap_or_default()
                        .into_iter()
                        .map(|layer| LambdaLayer {
                            arn: layer.arn.unwrap_or_default(),
                            code_size: layer.code_size,
                        })
                        .collect();

                    functions.push(LambdaFunction {
                        name: function_name,
                        arn: func.function_arn.unwrap_or_default(),
                        application,
                        description: func.description.unwrap_or_default(),
                        package_type: func
                            .package_type
                            .map(|p| format!("{:?}", p))
                            .unwrap_or_default(),
                        runtime: func.runtime.map(|r| format!("{:?}", r)).unwrap_or_default(),
                        architecture: func
                            .architectures
                            .and_then(|a| a.first().map(|arch| format!("{:?}", arch)))
                            .unwrap_or_default(),
                        code_size: func.code_size,
                        code_sha256: func.code_sha256.unwrap_or_default(),
                        memory_mb: func.memory_size.unwrap_or(0),
                        timeout_seconds: func.timeout.unwrap_or(0),
                        last_modified,
                        layers,
                    });
                }
            }

            next_marker = response.next_marker;
            if next_marker.is_none() {
                break;
            }
        }

        Ok(functions)
    }

    pub async fn list_applications(&self) -> Result<Vec<LambdaApplication>> {
        let client = self.config.cloudformation_client().await;

        let mut applications = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_stacks();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(stacks) = response.stack_summaries {
                for stack in stacks {
                    // Only include active stacks
                    let status = stack
                        .stack_status
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_default();
                    if status.contains("DELETE") {
                        continue;
                    }

                    applications.push(LambdaApplication {
                        name: stack.stack_name.unwrap_or_default(),
                        arn: stack.stack_id.unwrap_or_default(),
                        description: stack.template_description.unwrap_or_default(),
                        status,
                        last_modified: stack
                            .last_updated_time
                            .or(stack.creation_time)
                            .map(|dt| {
                                let timestamp = dt.secs();
                                let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
                                    .unwrap_or_default();
                                datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            })
                            .unwrap_or_default(),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(applications)
    }

    pub async fn list_versions(&self, function_name: &str) -> Result<Vec<LambdaVersion>> {
        let client = self.config.lambda_client().await;

        let mut versions = Vec::new();
        let mut next_marker: Option<String> = None;

        loop {
            let mut request = client
                .list_versions_by_function()
                .function_name(function_name)
                .max_items(100);
            if let Some(marker) = next_marker {
                request = request.marker(marker);
            }

            let response = request.send().await?;

            if let Some(vers) = response.versions {
                for ver in vers {
                    let last_modified = ver
                        .last_modified
                        .as_deref()
                        .map(|s| {
                            if let Ok(dt) =
                                chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3f%z")
                            {
                                dt.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                                dt.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            } else {
                                s.to_string()
                            }
                        })
                        .unwrap_or_default();

                    versions.push(LambdaVersion {
                        version: ver.version.unwrap_or_default(),
                        aliases: String::new(), // Will be populated below
                        description: ver.description.unwrap_or_default(),
                        last_modified,
                        architecture: ver
                            .architectures
                            .and_then(|a| a.first().map(|arch| format!("{:?}", arch)))
                            .unwrap_or_default(),
                    });
                }
            }

            next_marker = response.next_marker;
            if next_marker.is_none() {
                break;
            }
        }

        // Fetch aliases for all versions
        let aliases_response = client
            .list_aliases()
            .function_name(function_name)
            .send()
            .await?;

        if let Some(aliases) = aliases_response.aliases {
            for alias in aliases {
                let alias_name = alias.name.unwrap_or_default();

                // Add alias to primary version
                if let Some(version) = alias.function_version {
                    if let Some(ver) = versions.iter_mut().find(|v| v.version == version) {
                        if !ver.aliases.is_empty() {
                            ver.aliases.push_str(", ");
                        }
                        ver.aliases.push_str(&alias_name);
                    }
                }

                // Add alias to additional versions in routing config
                if let Some(routing_config) = alias.routing_config {
                    if let Some(additional_version_weights) =
                        routing_config.additional_version_weights
                    {
                        for (version, _weight) in additional_version_weights {
                            if let Some(ver) = versions.iter_mut().find(|v| v.version == version) {
                                if !ver.aliases.is_empty() {
                                    ver.aliases.push_str(", ");
                                }
                                ver.aliases.push_str(&alias_name);
                            }
                        }
                    }
                }
            }
        }

        Ok(versions)
    }
}

#[derive(Clone, Debug)]
pub struct LambdaApplication {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub status: String,
    pub last_modified: String,
}

impl LambdaClient {
    pub async fn list_aliases(&self, function_name: &str) -> Result<Vec<LambdaAlias>> {
        let client = self.config.lambda_client().await;
        let response = client
            .list_aliases()
            .function_name(function_name)
            .send()
            .await?;

        let mut aliases = Vec::new();
        if let Some(alias_list) = response.aliases {
            for alias in alias_list {
                let primary_version = alias.function_version.unwrap_or_default();

                // Check for additional versions in routing config
                let mut versions_str = primary_version.clone();
                if let Some(routing_config) = alias.routing_config {
                    if let Some(additional_version_weights) =
                        routing_config.additional_version_weights
                    {
                        for (version, weight) in additional_version_weights {
                            versions_str.push_str(&format!(
                                ", {} ({}%)",
                                version,
                                (weight * 100.0) as i32
                            ));
                        }
                    }
                }

                aliases.push(LambdaAlias {
                    name: alias.name.unwrap_or_default(),
                    versions: versions_str,
                    description: alias.description.unwrap_or_default(),
                });
            }
        }

        Ok(aliases)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_application_extraction_from_function_name() {
        // Test typical pattern: app-name-suffix
        let name = "storefront-studio-beta-api";
        let application = name.rsplit_once('-').map(|(prefix, _)| prefix).unwrap();
        assert_eq!(application, "storefront-studio-beta");

        // Test single dash
        let name = "myapp-api";
        let application = name.rsplit_once('-').map(|(prefix, _)| prefix).unwrap();
        assert_eq!(application, "myapp");

        // Test no dash
        let name = "simplefunction";
        let application = name.rsplit_once('-').map(|(prefix, _)| prefix);
        assert_eq!(application, None);

        // Test multiple dashes
        let name = "my-complex-app-name-worker";
        let application = name.rsplit_once('-').map(|(prefix, _)| prefix).unwrap();
        assert_eq!(application, "my-complex-app-name");
    }
}
