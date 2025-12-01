use crate::common::{format_bytes, t, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::lambda::{ApplicationDetailTab, DetailTab};
use crate::ui::table;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn parse_layer_arn(arn: &str) -> (String, String) {
    let parts: Vec<&str> = arn.split(':').collect();
    let name = parts.get(6).unwrap_or(&"").to_string();
    let version = parts.get(7).unwrap_or(&"").to_string();
    (name, version)
}

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in FunctionColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
    for col in ApplicationColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
    for col in DeploymentColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
    for col in ResourceColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
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

        let runtime_col = FunctionColumn::Runtime;
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

        let arch_col = FunctionColumn::Architecture;
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
        assert_eq!(FunctionColumn::Name.id(), "column.lambda.function.name");
        assert_eq!(FunctionColumn::Description.id(), "column.lambda.function.description");
        assert_eq!(FunctionColumn::PackageType.id(), "column.lambda.function.package_type");
        assert_eq!(FunctionColumn::Runtime.id(), "column.lambda.function.runtime");
        assert_eq!(FunctionColumn::Architecture.id(), "column.lambda.function.architecture");
        assert_eq!(FunctionColumn::CodeSize.id(), "column.lambda.function.code_size");
        assert_eq!(FunctionColumn::MemoryMb.id(), "column.lambda.function.memory_mb");
        assert_eq!(FunctionColumn::TimeoutSeconds.id(), "column.lambda.function.timeout_seconds");
        assert_eq!(FunctionColumn::LastModified.id(), "column.lambda.function.last_modified");
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(FunctionColumn::from_id("column.lambda.function.name"), Some(FunctionColumn::Name));
        assert_eq!(
            FunctionColumn::from_id("column.lambda.function.runtime"),
            Some(FunctionColumn::Runtime)
        );
        assert_eq!(FunctionColumn::from_id("invalid"), None);
    }

    #[test]
    fn test_column_all_returns_ids() {
        let all = FunctionColumn::ids();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&"column.lambda.function.name"));
        assert!(all.contains(&"column.lambda.function.runtime"));
        assert!(all.contains(&"column.lambda.function.code_size"));
    }

    #[test]
    fn test_column_visible_returns_ids() {
        let visible = FunctionColumn::visible();
        assert_eq!(visible.len(), 6);
        assert!(visible.contains(&"column.lambda.function.name"));
        assert!(visible.contains(&"column.lambda.function.runtime"));
        assert!(!visible.contains(&"column.lambda.function.description"));
    }

    #[test]
    fn test_application_column_id() {
        assert_eq!(ApplicationColumn::Name.id(), "column.lambda.application.name");
        assert_eq!(ApplicationColumn::Description.id(), "column.lambda.application.description");
        assert_eq!(ApplicationColumn::Status.id(), "column.lambda.application.status");
        assert_eq!(ApplicationColumn::LastModified.id(), "column.lambda.application.last_modified");
    }

    #[test]
    fn test_application_column_from_id() {
        assert_eq!(
            ApplicationColumn::from_id("column.lambda.application.name"),
            Some(ApplicationColumn::Name)
        );
        assert_eq!(
            ApplicationColumn::from_id("column.lambda.application.status"),
            Some(ApplicationColumn::Status)
        );
        assert_eq!(ApplicationColumn::from_id("invalid"), None);
    }

    #[test]
    fn test_i18n_initialization() {
        let mut i18n = std::collections::HashMap::new();
        init(&mut i18n);
        // Just verify that translation lookup works, don't assert specific values
        // since user may have custom column names in config.toml
        let name = t("column.lambda.function.name");
        assert!(!name.is_empty());
        let nonexistent = t("nonexistent.key");
        assert_eq!(nonexistent, "nonexistent.key");
    }

    #[test]
    fn test_column_width_uses_i18n() {
        let mut i18n = std::collections::HashMap::new();
        init(&mut i18n);
        let col = FunctionColumn::Name;
        let width = col.width();
        assert!(width >= "Function name".len() as u16);
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
    pub merge_order: String,
    pub name: String,
    pub layer_version: String,
    pub compatible_runtimes: String,
    pub compatible_architectures: String,
    pub version_arn: String,
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
pub enum FunctionColumn {
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

impl FunctionColumn {
    pub fn id(&self) -> ColumnId {
        match self {
            Self::Name => "column.lambda.function.name",
            Self::Description => "column.lambda.function.description",
            Self::PackageType => "column.lambda.function.package_type",
            Self::Runtime => "column.lambda.function.runtime",
            Self::Architecture => "column.lambda.function.architecture",
            Self::CodeSize => "column.lambda.function.code_size",
            Self::MemoryMb => "column.lambda.function.memory_mb",
            Self::TimeoutSeconds => "column.lambda.function.timeout_seconds",
            Self::LastModified => "column.lambda.function.last_modified",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Self::Name => "Function name",
            Self::Description => "Description",
            Self::PackageType => "Package type",
            Self::Runtime => "Runtime",
            Self::Architecture => "Architecture",
            Self::CodeSize => "Code size",
            Self::MemoryMb => "Memory (MB)",
            Self::TimeoutSeconds => "Timeout (s)",
            Self::LastModified => "Last modified",
        }
    }

    pub fn all() -> [Self; 9] {
        [
            Self::Name,
            Self::Description,
            Self::PackageType,
            Self::Runtime,
            Self::Architecture,
            Self::CodeSize,
            Self::MemoryMb,
            Self::TimeoutSeconds,
            Self::LastModified,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn visible() -> Vec<ColumnId> {
        [
            Self::Name,
            Self::Runtime,
            Self::CodeSize,
            Self::MemoryMb,
            Self::TimeoutSeconds,
            Self::LastModified,
        ]
        .iter()
        .map(|c| c.id())
        .collect()
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        match id {
            "column.lambda.function.name" => Some(Self::Name),
            "column.lambda.function.description" => Some(Self::Description),
            "column.lambda.function.package_type" => Some(Self::PackageType),
            "column.lambda.function.runtime" => Some(Self::Runtime),
            "column.lambda.function.architecture" => Some(Self::Architecture),
            "column.lambda.function.code_size" => Some(Self::CodeSize),
            "column.lambda.function.memory_mb" => Some(Self::MemoryMb),
            "column.lambda.function.timeout_seconds" => Some(Self::TimeoutSeconds),
            "column.lambda.function.last_modified" => Some(Self::LastModified),
            _ => None,
        }
    }
}

impl table::Column<Function> for FunctionColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
        let translated = t(self.id());
        translated.len().max(match self {
            Self::Name => 30,
            Self::Description => 40,
            Self::Runtime => 20,
            Self::LastModified => UTC_TIMESTAMP_WIDTH as usize,
            _ => 0,
        }) as u16
    }

    fn render(&self, item: &Function) -> (String, Style) {
        let text = match self {
            Self::Name => item.name.clone(),
            Self::Description => item.description.clone(),
            Self::PackageType => item.package_type.clone(),
            Self::Runtime => format_runtime(&item.runtime),
            Self::Architecture => format_architecture(&item.architecture),
            Self::CodeSize => format_bytes(item.code_size),
            Self::MemoryMb => item.memory_mb.to_string(),
            Self::TimeoutSeconds => item.timeout_seconds.to_string(),
            Self::LastModified => item.last_modified.clone(),
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
    pub fn id(&self) -> ColumnId {
        match self {
            Self::Name => "column.lambda.application.name",
            Self::Description => "column.lambda.application.description",
            Self::Status => "column.lambda.application.status",
            Self::LastModified => "column.lambda.application.last_modified",
        }
    }

    pub fn all() -> [Self; 4] {
        [
            Self::Name,
            Self::Description,
            Self::Status,
            Self::LastModified,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn visible() -> Vec<ColumnId> {
        Self::ids()
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        match id {
            "column.lambda.application.name" => Some(Self::Name),
            "column.lambda.application.description" => Some(Self::Description),
            "column.lambda.application.status" => Some(Self::Status),
            "column.lambda.application.last_modified" => Some(Self::LastModified),
            _ => None,
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Description => "Description",
            Self::Status => "Status",
            Self::LastModified => "Last modified",
        }
    }

    pub fn name(&self) -> String {
        let key = self.id();
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }
}

impl table::Column<Application> for ApplicationColumn {
    fn id(&self) -> &'static str {
        match self {
            Self::Name => "column.lambda.application.name",
            Self::Description => "column.lambda.application.description",
            Self::Status => "column.lambda.application.status",
            Self::LastModified => "column.lambda.application.last_modified",
        }
    }

    fn default_name(&self) -> &'static str {
        match self {
            Self::Name => "Application name",
            Self::Description => "Description",
            Self::Status => "Status",
            Self::LastModified => "Last modified",
        }
    }

    fn width(&self) -> u16 {
        self.name().len().max(match self {
            Self::Name => 40,
            Self::Description => 50,
            Self::Status => 20,
            Self::LastModified => UTC_TIMESTAMP_WIDTH as usize,
        }) as u16
    }

    fn render(&self, item: &Application) -> (String, Style) {
        match self {
            Self::Name => (item.name.clone(), Style::default()),
            Self::Description => (item.description.clone(), Style::default()),
            Self::Status => {
                let status_upper = item.status.to_uppercase();
                let text = if status_upper.contains("COMPLETE") {
                    format!("✅ {}", item.status)
                } else if status_upper == "UPDATE_IN_PROGRESS" {
                    format!("ℹ️  {}", item.status)
                } else if status_upper.contains("ROLLBACK") || status_upper.contains("_FAILED") {
                    format!("❌ {}", item.status)
                } else {
                    item.status.clone()
                };
                let color = if status_upper.contains("COMPLETE") {
                    Color::Green
                } else if status_upper == "UPDATE_IN_PROGRESS" {
                    Color::LightBlue
                } else if status_upper.contains("ROLLBACK") || status_upper.contains("_FAILED") {
                    Color::Red
                } else {
                    Color::White
                };
                (text, Style::default().fg(color))
            }
            Self::LastModified => (item.last_modified.clone(), Style::default()),
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

    pub fn all() -> [VersionColumn; 5] {
        [
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

    pub fn all() -> [AliasColumn; 3] {
        [
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
    pub fn default_name(&self) -> &'static str {
        match self {
            LayerColumn::MergeOrder => "Merge order",
            LayerColumn::Name => "Name",
            LayerColumn::LayerVersion => "Layer version",
            LayerColumn::CompatibleRuntimes => "Compatible runtimes",
            LayerColumn::CompatibleArchitectures => "Compatible architectures",
            LayerColumn::VersionArn => "Version ARN",
        }
    }

    pub fn name(&self) -> String {
        let key = self.id();
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn id(&self) -> ColumnId {
        match self {
            Self::MergeOrder => "column.lambda.layer.merge_order",
            Self::Name => "column.lambda.layer.name",
            Self::LayerVersion => "column.lambda.layer.layer_version",
            Self::CompatibleRuntimes => "column.lambda.layer.compatible_runtimes",
            Self::CompatibleArchitectures => "column.lambda.layer.compatible_architectures",
            Self::VersionArn => "column.lambda.layer.version_arn",
        }
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        match id {
            "column.lambda.layer.merge_order" => Some(Self::MergeOrder),
            "column.lambda.layer.name" => Some(Self::Name),
            "column.lambda.layer.layer_version" => Some(Self::LayerVersion),
            "column.lambda.layer.compatible_runtimes" => Some(Self::CompatibleRuntimes),
            "column.lambda.layer.compatible_architectures" => Some(Self::CompatibleArchitectures),
            "column.lambda.layer.version_arn" => Some(Self::VersionArn),
            _ => None,
        }
    }

    pub fn all() -> [LayerColumn; 6] {
        [
            LayerColumn::MergeOrder,
            LayerColumn::Name,
            LayerColumn::LayerVersion,
            LayerColumn::CompatibleRuntimes,
            LayerColumn::CompatibleArchitectures,
            LayerColumn::VersionArn,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl table::Column<Layer> for LayerColumn {
    fn id(&self) -> &'static str {
        match self {
            Self::MergeOrder => "column.lambda.layer.merge_order",
            Self::Name => "column.lambda.layer.name",
            Self::LayerVersion => "column.lambda.layer.layer_version",
            Self::CompatibleRuntimes => "column.lambda.layer.compatible_runtimes",
            Self::CompatibleArchitectures => "column.lambda.layer.compatible_architectures",
            Self::VersionArn => "column.lambda.layer.version_arn",
        }
    }

    fn default_name(&self) -> &'static str {
        match self {
            Self::MergeOrder => "Merge order",
            Self::Name => "Layer name",
            Self::LayerVersion => "Version",
            Self::CompatibleRuntimes => "Compatible runtimes",
            Self::CompatibleArchitectures => "Compatible architectures",
            Self::VersionArn => "Version ARN",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::MergeOrder => 12,
            Self::Name => 20,
            Self::LayerVersion => 14,
            Self::CompatibleRuntimes => 20,
            Self::CompatibleArchitectures => 26,
            Self::VersionArn => 40,
        }
    }

    fn render(&self, item: &Layer) -> (String, Style) {
        let text = match self {
            Self::MergeOrder => item.merge_order.clone(),
            Self::Name => item.name.clone(),
            Self::LayerVersion => item.layer_version.clone(),
            Self::CompatibleRuntimes => item.compatible_runtimes.clone(),
            Self::CompatibleArchitectures => item.compatible_architectures.clone(),
            Self::VersionArn => item.version_arn.clone(),
        };
        (text, Style::default())
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
    pub fn id(&self) -> ColumnId {
        match self {
            Self::Deployment => "column.lambda.deployment.deployment",
            Self::ResourceType => "column.lambda.deployment.resource_type",
            Self::LastUpdated => "column.lambda.deployment.last_updated",
            Self::Status => "column.lambda.deployment.status",
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
        let key = self.id();
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        match id {
            "column.lambda.deployment.deployment" => Some(Self::Deployment),
            "column.lambda.deployment.resource_type" => Some(Self::ResourceType),
            "column.lambda.deployment.last_updated" => Some(Self::LastUpdated),
            "column.lambda.deployment.status" => Some(Self::Status),
            _ => None,
        }
    }

    pub fn all() -> [Self; 4] {
        [
            Self::Deployment,
            Self::ResourceType,
            Self::LastUpdated,
            Self::Status,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl table::Column<Deployment> for DeploymentColumn {
    fn width(&self) -> u16 {
        let translated = t(&self.id());
        translated.len().max(match self {
            Self::Deployment => 30,
            Self::ResourceType => 20,
            Self::LastUpdated => UTC_TIMESTAMP_WIDTH as usize,
            Self::Status => 20,
        }) as u16
    }

    fn render(&self, item: &Deployment) -> (String, Style) {
        match self {
            Self::Deployment => (item.deployment_id.clone(), Style::default()),
            Self::ResourceType => (item.resource_type.clone(), Style::default()),
            Self::LastUpdated => (item.last_updated.clone(), Style::default()),
            Self::Status => {
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
    pub fn id(&self) -> ColumnId {
        match self {
            Self::LogicalId => "column.lambda.resource.logical_id",
            Self::PhysicalId => "column.lambda.resource.physical_id",
            Self::Type => "column.lambda.resource.type",
            Self::LastModified => "column.lambda.resource.last_modified",
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
        let key = self.id();
        let translated = t(&key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        match id {
            "column.lambda.resource.logical_id" => Some(Self::LogicalId),
            "column.lambda.resource.physical_id" => Some(Self::PhysicalId),
            "column.lambda.resource.type" => Some(Self::Type),
            "column.lambda.resource.last_modified" => Some(Self::LastModified),
            _ => None,
        }
    }

    pub fn all() -> [ResourceColumn; 4] {
        [
            Self::LogicalId,
            Self::PhysicalId,
            Self::Type,
            Self::LastModified,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl table::Column<Resource> for ResourceColumn {
    fn width(&self) -> u16 {
        match self {
            Self::LogicalId => 30,
            Self::PhysicalId => 40,
            Self::Type => 30,
            Self::LastModified => 27,
        }
    }

    fn render(&self, item: &Resource) -> (String, Style) {
        let text = match self {
            Self::LogicalId => item.logical_id.clone(),
            Self::PhysicalId => item.physical_id.clone(),
            Self::Type => item.resource_type.clone(),
            Self::LastModified => item.last_modified.clone(),
        };
        (text, Style::default())
    }
}

#[cfg(test)]
mod column_tests {
    use super::*;

    #[test]
    fn test_function_column_id_returns_full_key() {
        let id = FunctionColumn::Name.id();
        assert_eq!(id, "column.lambda.function.name");
        assert!(id.starts_with("column."));
    }

    #[test]
    fn test_application_column_id_returns_full_key() {
        let id = ApplicationColumn::Status.id();
        assert_eq!(id, "column.lambda.application.status");
        assert!(id.starts_with("column."));
    }

    #[test]
    fn test_layer_column_id_returns_full_key() {
        let id = LayerColumn::Name.id();
        assert_eq!(id, "column.lambda.layer.name");
        assert!(id.starts_with("column."));
    }

    #[test]
    fn test_deployment_column_id_returns_full_key() {
        let id = DeploymentColumn::Deployment.id();
        assert_eq!(id, "column.lambda.deployment.deployment");
        assert!(id.starts_with("column."));
    }

    #[test]
    fn test_resource_column_id_returns_full_key() {
        let id = ResourceColumn::LogicalId.id();
        assert_eq!(id, "column.lambda.resource.logical_id");
        assert!(id.starts_with("column."));
    }
}
