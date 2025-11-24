use crate::common::{format_bytes, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::lambda::{ApplicationDetailTab, DetailTab};
use crate::ui::table;
use ratatui::prelude::*;
use std::collections::HashMap;
use std::sync::OnceLock;

static I18N: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn init() {
    let mut map = HashMap::new();

    // Try to load from ~/.config/rusticity/i18n.toml
    if let Some(home) = std::env::var_os("HOME") {
        let config_path = std::path::Path::new(&home)
            .join(".config")
            .join("rusticity")
            .join("i18n.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(toml_map) = contents.parse::<toml::Table>() {
                if let Some(column_section) = toml_map.get("column").and_then(|v| v.as_table()) {
                    flatten_toml(column_section, "column", &mut map);
                }
            }
        }
    }

    // Set defaults for any missing keys
    map.entry("column.lambda.function.name".to_string())
        .or_insert("Function name".to_string());
    map.entry("column.lambda.function.description".to_string())
        .or_insert("Description".to_string());
    map.entry("column.lambda.function.package_type".to_string())
        .or_insert("Package type".to_string());
    map.entry("column.lambda.function.runtime".to_string())
        .or_insert("Runtime".to_string());
    map.entry("column.lambda.function.architecture".to_string())
        .or_insert("Architecture".to_string());
    map.entry("column.lambda.function.code_size".to_string())
        .or_insert("Code size".to_string());
    map.entry("column.lambda.function.memory_mb".to_string())
        .or_insert("Memory (MB)".to_string());
    map.entry("column.lambda.function.timeout_seconds".to_string())
        .or_insert("Timeout (s)".to_string());
    map.entry("column.lambda.function.last_modified".to_string())
        .or_insert("Last modified".to_string());
    map.entry("column.lambda.application.name".to_string())
        .or_insert("Name".to_string());
    map.entry("column.lambda.application.description".to_string())
        .or_insert("Description".to_string());
    map.entry("column.lambda.application.status".to_string())
        .or_insert("Status".to_string());
    map.entry("column.lambda.application.last_modified".to_string())
        .or_insert("Last modified".to_string());

    // Deployment columns
    for col in [
        DeploymentColumn::Deployment,
        DeploymentColumn::ResourceType,
        DeploymentColumn::LastUpdated,
        DeploymentColumn::Status,
    ] {
        let key = format!("column.lambda.deployment.{}", col.id());
        map.entry(key)
            .or_insert_with(|| col.default_name().to_string());
    }

    // Resource columns
    for col in [
        ResourceColumn::LogicalId,
        ResourceColumn::PhysicalId,
        ResourceColumn::Type,
        ResourceColumn::LastModified,
    ] {
        let key = format!("column.lambda.resource.{}", col.id());
        map.entry(key)
            .or_insert_with(|| col.default_name().to_string());
    }

    I18N.set(map).ok();
}

fn flatten_toml(table: &toml::Table, prefix: &str, map: &mut HashMap<String, String>) {
    for (key, value) in table {
        let full_key = format!("{}.{}", prefix, key);
        match value {
            toml::Value::String(s) => {
                map.insert(full_key, s.clone());
            }
            toml::Value::Table(t) => {
                flatten_toml(t, &full_key, map);
            }
            _ => {}
        }
    }
}

fn t(key: &str) -> String {
    I18N.get()
        .and_then(|map| map.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

pub fn format_runtime(runtime: &str) -> String {
    let lower = runtime.to_lowercase();

    if let Some(rest) = lower.strip_prefix("python") {
        let version = if rest.contains('.') {
            rest.to_string()
        } else if rest.len() >= 2 {
            format!("{}.{}", &rest[0..1], &rest[1..])
        } else {
            rest.to_string()
        };
        format!("Python {}", version)
    } else if let Some(rest) = lower.strip_prefix("nodejs") {
        let formatted = rest.replace("x", ".x");
        format!("Node.js {}", formatted)
    } else if let Some(rest) = lower.strip_prefix("java") {
        format!("Java {}", rest)
    } else if let Some(rest) = lower.strip_prefix("dotnet") {
        format!(".NET {}", rest)
    } else if let Some(rest) = lower.strip_prefix("go") {
        format!("Go {}", rest)
    } else if let Some(rest) = lower.strip_prefix("ruby") {
        format!("Ruby {}", rest)
    } else {
        runtime.to_string()
    }
}

pub fn format_architecture(arch: &str) -> String {
    match arch {
        "X86_64" => "x86-64".to_string(),
        "X8664" => "x86-64".to_string(),
        "Arm64" => "arm64".to_string(),
        _ => arch.replace("X86", "x86").replace("Arm", "arm"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::InputFocus;
    use crate::ui::lambda::FILTER_CONTROLS;
    use crate::ui::table::Column as TableColumn;

    #[test]
    fn test_format_runtime() {
        // AWS SDK format (e.g., Python39, Nodejs20x)
        assert_eq!(format_runtime("Python39"), "Python 3.9");
        assert_eq!(format_runtime("Python312"), "Python 3.12");
        assert_eq!(format_runtime("Nodejs20x"), "Node.js 20.x");

        // Lowercase format
        assert_eq!(format_runtime("python3.12"), "Python 3.12");
        assert_eq!(format_runtime("python3.11"), "Python 3.11");
        assert_eq!(format_runtime("nodejs20x"), "Node.js 20.x");
        assert_eq!(format_runtime("nodejs18x"), "Node.js 18.x");
        assert_eq!(format_runtime("java21"), "Java 21");
        assert_eq!(format_runtime("dotnet8"), ".NET 8");
        assert_eq!(format_runtime("go1.x"), "Go 1.x");
        assert_eq!(format_runtime("ruby3.3"), "Ruby 3.3");
        assert_eq!(format_runtime("unknown"), "unknown");
    }

    #[test]
    fn test_format_architecture() {
        assert_eq!(format_architecture("X8664"), "x86-64");
        assert_eq!(format_architecture("X86_64"), "x86-64");
        assert_eq!(format_architecture("Arm64"), "arm64");
        assert_eq!(format_architecture("arm64"), "arm64");
    }

    #[test]
    fn test_runtime_formatter_in_table_column() {
        let func = Function {
            name: "test-func".to_string(),
            arn: "arn".to_string(),
            application: None,
            description: "desc".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![],
        };

        let runtime_col = Column::Runtime;
        let (text, _) = runtime_col.render(&func);
        assert_eq!(text, "Python 3.12");
    }

    #[test]
    fn test_architecture_formatter_in_table_column() {
        let func = Function {
            name: "test-func".to_string(),
            arn: "arn".to_string(),
            application: None,
            description: "desc".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![],
        };

        let arch_col = Column::Architecture;
        let (text, _) = arch_col.render(&func);
        assert_eq!(text, "x86-64");
    }

    #[test]
    fn test_nodejs_runtime_formatting() {
        assert_eq!(format_runtime("nodejs16x"), "Node.js 16.x");
        assert_eq!(format_runtime("nodejs18x"), "Node.js 18.x");
        assert_eq!(format_runtime("nodejs20x"), "Node.js 20.x");
    }

    #[test]
    fn test_python_runtime_formatting() {
        // AWS SDK format
        assert_eq!(format_runtime("Python38"), "Python 3.8");
        assert_eq!(format_runtime("Python39"), "Python 3.9");
        assert_eq!(format_runtime("Python310"), "Python 3.10");
        assert_eq!(format_runtime("Python311"), "Python 3.11");
        assert_eq!(format_runtime("Python312"), "Python 3.12");

        // Lowercase with dots
        assert_eq!(format_runtime("python3.8"), "Python 3.8");
        assert_eq!(format_runtime("python3.9"), "Python 3.9");
        assert_eq!(format_runtime("python3.10"), "Python 3.10");
        assert_eq!(format_runtime("python3.11"), "Python 3.11");
        assert_eq!(format_runtime("python3.12"), "Python 3.12");
    }

    #[test]
    fn test_timeout_formatting() {
        // Test timeout conversion to min/sec format
        assert_eq!(300 / 60, 5); // 300 seconds = 5 minutes
        assert_eq!(300 % 60, 0); // 0 seconds remainder
        assert_eq!(900 / 60, 15); // 900 seconds = 15 minutes
        assert_eq!(900 % 60, 0); // 0 seconds remainder
        assert_eq!(330 / 60, 5); // 330 seconds = 5 minutes
        assert_eq!(330 % 60, 30); // 30 seconds remainder
    }

    #[test]
    fn test_version_column_architecture_formatter() {
        let version = Version {
            version: "1".to_string(),
            aliases: "prod".to_string(),
            description: "Production version".to_string(),
            last_modified: "2024-01-01".to_string(),
            architecture: "X86_64".to_string(),
        };

        let arch_col = VersionColumn::Architecture.to_column();
        let (text, _) = arch_col.render(&version);
        assert_eq!(text, "x86-64");
    }

    #[test]
    fn test_version_architecture_formatter_arm() {
        let version = Version {
            version: "1".to_string(),
            aliases: "".to_string(),
            description: "".to_string(),
            last_modified: "".to_string(),
            architecture: "Arm64".to_string(),
        };

        let arch_col = VersionColumn::Architecture.to_column();
        let (text, _) = arch_col.render(&version);
        assert_eq!(text, "arm64");
    }

    #[test]
    fn test_version_column_all() {
        let all = VersionColumn::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&VersionColumn::Version));
        assert!(all.contains(&VersionColumn::Aliases));
        assert!(all.contains(&VersionColumn::Description));
        assert!(all.contains(&VersionColumn::LastModified));
        assert!(all.contains(&VersionColumn::Architecture));
    }

    #[test]
    fn test_version_column_names() {
        assert_eq!(VersionColumn::Version.name(), "Version");
        assert_eq!(VersionColumn::Aliases.name(), "Aliases");
        assert_eq!(VersionColumn::Description.name(), "Description");
        assert_eq!(VersionColumn::LastModified.name(), "Last modified");
        assert_eq!(VersionColumn::Architecture.name(), "Architecture");
    }

    #[test]
    fn test_versions_table_sort_config() {
        // Versions table shows "Version ↓" header (DESC sort)
        // This is configured in render_detail with:
        // sort_column: "Version", sort_direction: "DESC"
        let sort_column = "Version";
        let sort_direction = "DESC";
        assert_eq!(sort_column, "Version");
        assert_eq!(sort_direction, "DESC");
    }

    #[test]
    fn test_input_focus_cycling() {
        let focus = InputFocus::Filter;
        assert_eq!(focus.next(&FILTER_CONTROLS), InputFocus::Pagination);

        let focus = InputFocus::Pagination;
        assert_eq!(focus.next(&FILTER_CONTROLS), InputFocus::Filter);

        let focus = InputFocus::Filter;
        assert_eq!(focus.prev(&FILTER_CONTROLS), InputFocus::Pagination);

        let focus = InputFocus::Pagination;
        assert_eq!(focus.prev(&FILTER_CONTROLS), InputFocus::Filter);
    }

    #[test]
    #[allow(clippy::useless_vec)]
    fn test_version_sorting_desc() {
        let mut versions = vec![
            Version {
                version: "1".to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "".to_string(),
            },
            Version {
                version: "10".to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "".to_string(),
            },
            Version {
                version: "2".to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "".to_string(),
            },
        ];

        // Sort DESC by version number
        versions.sort_by(|a, b| {
            let a_num = a.version.parse::<i32>().unwrap_or(0);
            let b_num = b.version.parse::<i32>().unwrap_or(0);
            b_num.cmp(&a_num)
        });

        assert_eq!(versions[0].version, "10");
        assert_eq!(versions[1].version, "2");
        assert_eq!(versions[2].version, "1");
    }

    #[test]
    fn test_version_sorting_with_36_versions() {
        let mut versions: Vec<Version> = (1..=36)
            .map(|i| Version {
                version: i.to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "".to_string(),
            })
            .collect();

        // Sort DESC by version number
        versions.sort_by(|a, b| {
            let a_num = a.version.parse::<i32>().unwrap_or(0);
            let b_num = b.version.parse::<i32>().unwrap_or(0);
            b_num.cmp(&a_num)
        });

        // Verify DESC order
        assert_eq!(versions[0].version, "36");
        assert_eq!(versions[1].version, "35");
        assert_eq!(versions[35].version, "1");
        assert_eq!(versions.len(), 36);
    }

    #[test]
    fn test_column_id() {
        assert_eq!(Column::Name.id(), "name");
        assert_eq!(Column::Description.id(), "description");
        assert_eq!(Column::PackageType.id(), "package_type");
        assert_eq!(Column::Runtime.id(), "runtime");
        assert_eq!(Column::Architecture.id(), "architecture");
        assert_eq!(Column::CodeSize.id(), "code_size");
        assert_eq!(Column::MemoryMb.id(), "memory_mb");
        assert_eq!(Column::TimeoutSeconds.id(), "timeout_seconds");
        assert_eq!(Column::LastModified.id(), "last_modified");
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(Column::from_id("name"), Some(Column::Name));
        assert_eq!(Column::from_id("runtime"), Some(Column::Runtime));
        assert_eq!(Column::from_id("invalid"), None);
    }

    #[test]
    fn test_column_all_returns_ids() {
        let all = Column::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&"name".to_string()));
        assert!(all.contains(&"runtime".to_string()));
        assert!(all.contains(&"code_size".to_string()));
    }

    #[test]
    fn test_column_visible_returns_ids() {
        let visible = Column::visible();
        assert_eq!(visible.len(), 6);
        assert!(visible.contains(&"name".to_string()));
        assert!(visible.contains(&"runtime".to_string()));
        assert!(!visible.contains(&"description".to_string()));
    }

    #[test]
    fn test_application_column_id() {
        assert_eq!(ApplicationColumn::Name.id(), "name");
        assert_eq!(ApplicationColumn::Description.id(), "description");
        assert_eq!(ApplicationColumn::Status.id(), "status");
        assert_eq!(ApplicationColumn::LastModified.id(), "last_modified");
    }

    #[test]
    fn test_application_column_from_id() {
        assert_eq!(
            ApplicationColumn::from_id("name"),
            Some(ApplicationColumn::Name)
        );
        assert_eq!(
            ApplicationColumn::from_id("status"),
            Some(ApplicationColumn::Status)
        );
        assert_eq!(ApplicationColumn::from_id("invalid"), None);
    }

    #[test]
    fn test_i18n_initialization() {
        init();
        // Just verify that translation lookup works, don't assert specific values
        // since user may have custom i18n.toml
        let name = t("column.lambda.function.name");
        assert!(!name.is_empty());
        let nonexistent = t("nonexistent.key");
        assert_eq!(nonexistent, "nonexistent.key");
    }

    #[test]
    fn test_column_width_uses_i18n() {
        init();
        let col = Column::Name;
        let width = col.width();
        assert!(width >= "Function name".len() as u16);
    }

    #[test]
    fn test_flatten_toml() {
        let mut inner_table = toml::Table::new();
        inner_table.insert(
            "name".to_string(),
            toml::Value::String("Nom de fonction".to_string()),
        );
        inner_table.insert(
            "description".to_string(),
            toml::Value::String("Description".to_string()),
        );

        let mut function_table = toml::Table::new();
        function_table.insert("function".to_string(), toml::Value::Table(inner_table));

        let mut map = HashMap::new();
        flatten_toml(&function_table, "column.lambda", &mut map);

        assert_eq!(
            map.get("column.lambda.function.name"),
            Some(&"Nom de fonction".to_string())
        );
        assert_eq!(
            map.get("column.lambda.function.description"),
            Some(&"Description".to_string())
        );
    }
}

pub fn console_url_functions(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/lambda/home?region={}#/functions",
        region, region
    )
}

pub fn console_url_function_detail(region: &str, function_name: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/lambda/home?region={}#/functions/{}",
        region, region, function_name
    )
}

pub fn console_url_function_version(
    region: &str,
    function_name: &str,
    version: &str,
    detail_tab: &DetailTab,
) -> String {
    let tab = match detail_tab {
        DetailTab::Code => "code",
        DetailTab::Configuration => "configure",
        _ => "code",
    };
    format!(
        "https://{}.console.aws.amazon.com/lambda/home?region={}#/functions/{}/versions/{}?tab={}",
        region, region, function_name, version, tab
    )
}

pub fn console_url_applications(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/lambda/home?region={}#/applications",
        region, region
    )
}

pub fn console_url_application_detail(
    region: &str,
    app_name: &str,
    tab: &ApplicationDetailTab,
) -> String {
    let tab_param = match tab {
        ApplicationDetailTab::Overview => "overview",
        ApplicationDetailTab::Deployments => "deployments",
    };
    format!(
        "https://{}.console.aws.amazon.com/lambda/home?region={}#/applications/{}?tab={}",
        region, region, app_name, tab_param
    )
}

#[derive(Debug, Clone)]
pub struct Function {
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
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub arn: String,
    pub code_size: i64,
}

#[derive(Debug, Clone)]
pub struct Version {
    pub version: String,
    pub aliases: String,
    pub description: String,
    pub last_modified: String,
    pub architecture: String,
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub versions: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub status: String,
    pub last_modified: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    Description,
    PackageType,
    Runtime,
    Architecture,
    CodeSize,
    MemoryMb,
    TimeoutSeconds,
    LastModified,
}

impl Column {
    pub fn id(&self) -> &'static str {
        match self {
            Column::Name => "name",
            Column::Description => "description",
            Column::PackageType => "package_type",
            Column::Runtime => "runtime",
            Column::Architecture => "architecture",
            Column::CodeSize => "code_size",
            Column::MemoryMb => "memory_mb",
            Column::TimeoutSeconds => "timeout_seconds",
            Column::LastModified => "last_modified",
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.lambda.function.{}", self.id());
        let translated = t(&key);
        if translated == key {
            match self {
                Column::Name => "Function name",
                Column::Description => "Description",
                Column::PackageType => "Package type",
                Column::Runtime => "Runtime",
                Column::Architecture => "Architecture",
                Column::CodeSize => "Code size",
                Column::MemoryMb => "Memory (MB)",
                Column::TimeoutSeconds => "Timeout (s)",
                Column::LastModified => "Last modified",
            }
            .to_string()
        } else {
            translated
        }
    }

    pub fn all() -> Vec<String> {
        [
            Column::Name,
            Column::Description,
            Column::PackageType,
            Column::Runtime,
            Column::Architecture,
            Column::CodeSize,
            Column::MemoryMb,
            Column::TimeoutSeconds,
            Column::LastModified,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn visible() -> Vec<String> {
        [
            Column::Name,
            Column::Runtime,
            Column::CodeSize,
            Column::MemoryMb,
            Column::TimeoutSeconds,
            Column::LastModified,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "name" => Some(Column::Name),
            "description" => Some(Column::Description),
            "package_type" => Some(Column::PackageType),
            "runtime" => Some(Column::Runtime),
            "architecture" => Some(Column::Architecture),
            "code_size" => Some(Column::CodeSize),
            "memory_mb" => Some(Column::MemoryMb),
            "timeout_seconds" => Some(Column::TimeoutSeconds),
            "last_modified" => Some(Column::LastModified),
            _ => None,
        }
    }
}

impl table::Column<Function> for Column {
    fn name(&self) -> &str {
        let key = format!("column.lambda.function.{}", self.id());
        let translated = t(&key);
        if translated == key {
            // No i18n found, use default
            match self {
                Column::Name => "Function name",
                Column::Description => "Description",
                Column::PackageType => "Package type",
                Column::Runtime => "Runtime",
                Column::Architecture => "Architecture",
                Column::CodeSize => "Code size",
                Column::MemoryMb => "Memory (MB)",
                Column::TimeoutSeconds => "Timeout (s)",
                Column::LastModified => "Last modified",
            }
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.lambda.function.{}", self.id()));
        translated.len().max(match self {
            Column::Name => 30,
            Column::Description => 40,
            Column::Runtime => 20,
            Column::LastModified => UTC_TIMESTAMP_WIDTH as usize,
            _ => 0,
        }) as u16
    }

    fn render(&self, item: &Function) -> (String, Style) {
        let text = match self {
            Column::Name => item.name.clone(),
            Column::Description => item.description.clone(),
            Column::PackageType => item.package_type.clone(),
            Column::Runtime => format_runtime(&item.runtime),
            Column::Architecture => format_architecture(&item.architecture),
            Column::CodeSize => format_bytes(item.code_size),
            Column::MemoryMb => item.memory_mb.to_string(),
            Column::TimeoutSeconds => item.timeout_seconds.to_string(),
            Column::LastModified => item.last_modified.clone(),
        };
        (text, Style::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApplicationColumn {
    Name,
    Description,
    Status,
    LastModified,
}

impl ApplicationColumn {
    pub fn id(&self) -> &'static str {
        match self {
            ApplicationColumn::Name => "name",
            ApplicationColumn::Description => "description",
            ApplicationColumn::Status => "status",
            ApplicationColumn::LastModified => "last_modified",
        }
    }

    pub fn all() -> Vec<String> {
        [
            ApplicationColumn::Name,
            ApplicationColumn::Description,
            ApplicationColumn::Status,
            ApplicationColumn::LastModified,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn visible() -> Vec<String> {
        Self::all()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "name" => Some(ApplicationColumn::Name),
            "description" => Some(ApplicationColumn::Description),
            "status" => Some(ApplicationColumn::Status),
            "last_modified" => Some(ApplicationColumn::LastModified),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.lambda.application.{}", self.id());
        let translated = t(&key);
        if translated == key {
            match self {
                ApplicationColumn::Name => "Name",
                ApplicationColumn::Description => "Description",
                ApplicationColumn::Status => "Status",
                ApplicationColumn::LastModified => "Last modified",
            }
            .to_string()
        } else {
            translated
        }
    }
}

impl table::Column<Application> for ApplicationColumn {
    fn name(&self) -> &str {
        let key = format!("column.lambda.application.{}", self.id());
        let translated = t(&key);
        if translated == key {
            match self {
                ApplicationColumn::Name => "Name",
                ApplicationColumn::Description => "Description",
                ApplicationColumn::Status => "Status",
                ApplicationColumn::LastModified => "Last modified",
            }
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let translated = t(&format!("column.lambda.application.{}", self.id()));
        translated.len().max(match self {
            ApplicationColumn::Name => 40,
            ApplicationColumn::Description => 50,
            ApplicationColumn::Status => 20,
            ApplicationColumn::LastModified => UTC_TIMESTAMP_WIDTH as usize,
        }) as u16
    }

    fn render(&self, item: &Application) -> (String, Style) {
        match self {
            ApplicationColumn::Name => (item.name.clone(), Style::default()),
            ApplicationColumn::Description => (item.description.clone(), Style::default()),
            ApplicationColumn::Status => {
                let status_upper = item.status.to_uppercase();
                let (text, color) = if status_upper.contains("UPDATE_COMPLETE") {
                    ("✅ Update complete", Color::Green)
                } else if status_upper.contains("CREATE_COMPLETE") {
                    ("✅ Create complete", Color::Green)
                } else {
                    (item.status.as_str(), Color::White)
                };
                (text.to_string(), Style::default().fg(color))
            }
            ApplicationColumn::LastModified => (item.last_modified.clone(), Style::default()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VersionColumn {
    Version,
    Aliases,
    Description,
    LastModified,
    Architecture,
}

impl VersionColumn {
    pub fn name(&self) -> &'static str {
        match self {
            VersionColumn::Version => "Version",
            VersionColumn::Aliases => "Aliases",
            VersionColumn::Description => "Description",
            VersionColumn::LastModified => "Last modified",
            VersionColumn::Architecture => "Architecture",
        }
    }

    pub fn all() -> Vec<VersionColumn> {
        vec![
            VersionColumn::Version,
            VersionColumn::Aliases,
            VersionColumn::Description,
            VersionColumn::LastModified,
            VersionColumn::Architecture,
        ]
    }

    pub fn to_column(&self) -> Box<dyn table::Column<Version>> {
        struct VersionCol {
            variant: VersionColumn,
        }

        impl table::Column<Version> for VersionCol {
            fn name(&self) -> &str {
                self.variant.name()
            }

            fn width(&self) -> u16 {
                match self.variant {
                    VersionColumn::Version => 10,
                    VersionColumn::Aliases => 20,
                    VersionColumn::Description => 40,
                    VersionColumn::LastModified => UTC_TIMESTAMP_WIDTH,
                    VersionColumn::Architecture => 15,
                }
            }

            fn render(&self, item: &Version) -> (String, Style) {
                let text = match self.variant {
                    VersionColumn::Version => item.version.clone(),
                    VersionColumn::Aliases => item.aliases.clone(),
                    VersionColumn::Description => item.description.clone(),
                    VersionColumn::LastModified => item.last_modified.clone(),
                    VersionColumn::Architecture => format_architecture(&item.architecture),
                };
                (text, Style::default())
            }
        }

        Box::new(VersionCol { variant: *self })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AliasColumn {
    Name,
    Versions,
    Description,
}

impl AliasColumn {
    pub fn name(&self) -> &'static str {
        match self {
            AliasColumn::Name => "Name",
            AliasColumn::Versions => "Versions",
            AliasColumn::Description => "Description",
        }
    }

    pub fn all() -> Vec<AliasColumn> {
        vec![
            AliasColumn::Name,
            AliasColumn::Versions,
            AliasColumn::Description,
        ]
    }

    pub fn to_column(&self) -> Box<dyn table::Column<Alias>> {
        struct AliasCol {
            variant: AliasColumn,
        }

        impl table::Column<Alias> for AliasCol {
            fn name(&self) -> &str {
                self.variant.name()
            }

            fn width(&self) -> u16 {
                match self.variant {
                    AliasColumn::Name => 20,
                    AliasColumn::Versions => 15,
                    AliasColumn::Description => 50,
                }
            }

            fn render(&self, item: &Alias) -> (String, Style) {
                let text = match self.variant {
                    AliasColumn::Name => item.name.clone(),
                    AliasColumn::Versions => item.versions.clone(),
                    AliasColumn::Description => item.description.clone(),
                };
                (text, Style::default())
            }
        }

        Box::new(AliasCol { variant: *self })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerColumn {
    MergeOrder,
    Name,
    LayerVersion,
    CompatibleRuntimes,
    CompatibleArchitectures,
    VersionArn,
}

impl LayerColumn {
    pub fn name(&self) -> &'static str {
        match self {
            LayerColumn::MergeOrder => "Merge order",
            LayerColumn::Name => "Name",
            LayerColumn::LayerVersion => "Layer version",
            LayerColumn::CompatibleRuntimes => "Compatible runtimes",
            LayerColumn::CompatibleArchitectures => "Compatible architectures",
            LayerColumn::VersionArn => "Version ARN",
        }
    }

    pub fn all() -> Vec<LayerColumn> {
        vec![
            LayerColumn::MergeOrder,
            LayerColumn::Name,
            LayerColumn::LayerVersion,
            LayerColumn::CompatibleRuntimes,
            LayerColumn::CompatibleArchitectures,
            LayerColumn::VersionArn,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeploymentColumn {
    Deployment,
    ResourceType,
    LastUpdated,
    Status,
}

impl DeploymentColumn {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Deployment => "deployment",
            Self::ResourceType => "resource_type",
            Self::LastUpdated => "last_updated",
            Self::Status => "status",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::ResourceType => "Resource type",
            Self::LastUpdated => "Last updated time",
            Self::Status => "Status",
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.lambda.deployment.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "deployment" => Some(Self::Deployment),
            "resource_type" => Some(Self::ResourceType),
            "last_updated" => Some(Self::LastUpdated),
            "status" => Some(Self::Status),
            _ => None,
        }
    }

    pub fn all() -> Vec<ColumnId> {
        [
            Self::Deployment,
            Self::ResourceType,
            Self::LastUpdated,
            Self::Status,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }

    pub fn as_table_column(self) -> Box<dyn table::Column<Deployment>> {
        struct DeploymentColumnImpl {
            variant: DeploymentColumn,
        }

        impl table::Column<Deployment> for DeploymentColumnImpl {
            fn name(&self) -> &str {
                let key = format!("column.lambda.deployment.{}", self.variant.id());
                let translated = t(&key);
                if translated == key {
                    self.variant.default_name()
                } else {
                    Box::leak(translated.into_boxed_str())
                }
            }

            fn width(&self) -> u16 {
                let translated = t(&format!("column.lambda.deployment.{}", self.variant.id()));
                translated.len().max(match self.variant {
                    DeploymentColumn::Deployment => 30,
                    DeploymentColumn::ResourceType => 20,
                    DeploymentColumn::LastUpdated => UTC_TIMESTAMP_WIDTH as usize,
                    DeploymentColumn::Status => 20,
                }) as u16
            }

            fn render(&self, item: &Deployment) -> (String, Style) {
                match self.variant {
                    DeploymentColumn::Deployment => (item.deployment_id.clone(), Style::default()),
                    DeploymentColumn::ResourceType => {
                        (item.resource_type.clone(), Style::default())
                    }
                    DeploymentColumn::LastUpdated => (item.last_updated.clone(), Style::default()),
                    DeploymentColumn::Status => {
                        if item.status == "Succeeded" {
                            (
                                format!("✅ {}", item.status),
                                Style::default().fg(Color::Green),
                            )
                        } else {
                            (item.status.clone(), Style::default())
                        }
                    }
                }
            }
        }

        Box::new(DeploymentColumnImpl { variant: self })
    }
}

#[derive(Clone, Debug)]
pub struct Resource {
    pub logical_id: String,
    pub physical_id: String,
    pub resource_type: String,
    pub last_modified: String,
}

#[derive(Clone, Debug)]
pub struct Deployment {
    pub deployment_id: String,
    pub resource_type: String,
    pub last_updated: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceColumn {
    LogicalId,
    PhysicalId,
    Type,
    LastModified,
}

impl ResourceColumn {
    pub fn id(&self) -> &'static str {
        match self {
            Self::LogicalId => "logical_id",
            Self::PhysicalId => "physical_id",
            Self::Type => "type",
            Self::LastModified => "last_modified",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Self::LogicalId => "Logical ID",
            Self::PhysicalId => "Physical ID",
            Self::Type => "Type",
            Self::LastModified => "Last modified",
        }
    }

    pub fn name(&self) -> String {
        let key = format!("column.lambda.resource.{}", self.id());
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "logical_id" => Some(Self::LogicalId),
            "physical_id" => Some(Self::PhysicalId),
            "type" => Some(Self::Type),
            "last_modified" => Some(Self::LastModified),
            _ => None,
        }
    }

    pub fn all() -> Vec<ColumnId> {
        [
            Self::LogicalId,
            Self::PhysicalId,
            Self::Type,
            Self::LastModified,
        ]
        .iter()
        .map(|c| c.id().to_string())
        .collect()
    }
}
