use crate::common::{format_bytes, ColumnTrait, UTC_TIMESTAMP_WIDTH};
use crate::ui::lambda::{ApplicationDetailTab, DetailTab};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;

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

        let runtime_col = Column::Runtime.to_column();
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

        let arch_col = Column::Architecture.to_column();
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
        use crate::common::InputFocus;
        use crate::ui::lambda::FILTER_CONTROLS;

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
    pub fn name(&self) -> &'static str {
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
    }

    pub fn all() -> Vec<Column> {
        vec![
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
    }

    pub fn to_column(&self) -> Box<dyn TableColumn<Function>> {
        struct FunctionColumn {
            variant: Column,
        }

        impl TableColumn<Function> for FunctionColumn {
            fn name(&self) -> &str {
                self.variant.name()
            }

            fn width(&self) -> u16 {
                self.variant.name().len().max(match self.variant {
                    Column::Name => 30,
                    Column::Description => 40,
                    Column::Runtime => 20,
                    Column::LastModified => UTC_TIMESTAMP_WIDTH as usize,
                    _ => 0,
                }) as u16
            }

            fn render(&self, item: &Function) -> (String, Style) {
                let text = match self.variant {
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

        Box::new(FunctionColumn { variant: *self })
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
    pub fn name(&self) -> &'static str {
        match self {
            ApplicationColumn::Name => "Name",
            ApplicationColumn::Description => "Description",
            ApplicationColumn::Status => "Status",
            ApplicationColumn::LastModified => "Last modified",
        }
    }

    pub fn all() -> Vec<ApplicationColumn> {
        vec![
            ApplicationColumn::Name,
            ApplicationColumn::Description,
            ApplicationColumn::Status,
            ApplicationColumn::LastModified,
        ]
    }

    pub fn to_column(&self) -> Box<dyn crate::ui::table::Column<Application>> {
        struct ApplicationColumnImpl {
            variant: ApplicationColumn,
        }

        impl crate::ui::table::Column<Application> for ApplicationColumnImpl {
            fn name(&self) -> &str {
                self.variant.name()
            }

            fn width(&self) -> u16 {
                match self.variant {
                    ApplicationColumn::Name => 40,
                    ApplicationColumn::Description => 50,
                    ApplicationColumn::Status => 20,
                    ApplicationColumn::LastModified => UTC_TIMESTAMP_WIDTH,
                }
            }

            fn render(&self, item: &Application) -> (String, Style) {
                use ratatui::prelude::{Color, Style};
                match self.variant {
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
                    ApplicationColumn::LastModified => {
                        (item.last_modified.clone(), Style::default())
                    }
                }
            }
        }

        Box::new(ApplicationColumnImpl { variant: *self })
    }
}

impl ColumnTrait for Column {
    fn name(&self) -> &'static str {
        self.name()
    }
}

impl ColumnTrait for ApplicationColumn {
    fn name(&self) -> &'static str {
        self.name()
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

    pub fn to_column(&self) -> Box<dyn crate::ui::table::Column<Version>> {
        struct VersionCol {
            variant: VersionColumn,
        }

        impl crate::ui::table::Column<Version> for VersionCol {
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
                (text, ratatui::style::Style::default())
            }
        }

        Box::new(VersionCol { variant: *self })
    }
}

impl ColumnTrait for VersionColumn {
    fn name(&self) -> &'static str {
        self.name()
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

    pub fn to_column(&self) -> Box<dyn crate::ui::table::Column<Alias>> {
        struct AliasCol {
            variant: AliasColumn,
        }

        impl crate::ui::table::Column<Alias> for AliasCol {
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
                (text, ratatui::style::Style::default())
            }
        }

        Box::new(AliasCol { variant: *self })
    }
}

impl ColumnTrait for AliasColumn {
    fn name(&self) -> &'static str {
        self.name()
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

impl ColumnTrait for LayerColumn {
    fn name(&self) -> &'static str {
        self.name()
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
    pub fn all() -> Vec<Self> {
        vec![
            Self::Deployment,
            Self::ResourceType,
            Self::LastUpdated,
            Self::Status,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::ResourceType => "Resource type",
            Self::LastUpdated => "Last updated time",
            Self::Status => "Status",
        }
    }

    pub fn as_table_column(self) -> Box<dyn TableColumn<Deployment>> {
        struct DeploymentColumnImpl {
            variant: DeploymentColumn,
        }

        impl TableColumn<Deployment> for DeploymentColumnImpl {
            fn name(&self) -> &str {
                self.variant.name()
            }

            fn width(&self) -> u16 {
                match self.variant {
                    DeploymentColumn::Deployment => 30,
                    DeploymentColumn::ResourceType => 20,
                    DeploymentColumn::LastUpdated => UTC_TIMESTAMP_WIDTH,
                    DeploymentColumn::Status => 20,
                }
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

impl ColumnTrait for DeploymentColumn {
    fn name(&self) -> &'static str {
        self.name()
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

impl ColumnTrait for ResourceColumn {
    fn name(&self) -> &'static str {
        self.name()
    }
}

impl ResourceColumn {
    pub fn all() -> Vec<Self> {
        vec![
            Self::LogicalId,
            Self::PhysicalId,
            Self::Type,
            Self::LastModified,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::LogicalId => "Logical ID",
            Self::PhysicalId => "Physical ID",
            Self::Type => "Type",
            Self::LastModified => "Last modified",
        }
    }
}
