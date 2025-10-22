use crate::common::SortDirection;
use crate::ui::active_border;
use crate::ui::table::{render_table, Column as TableColumn, TableConfig};
use ratatui::{prelude::*, widgets::*};

pub struct Profile {
    pub name: String,
    pub region: Option<String>,
    pub account: Option<String>,
    pub role_arn: Option<String>,
    pub source_profile: Option<String>,
}

impl Profile {
    pub fn load_all() -> Vec<Self> {
        let mut profiles = Vec::new();
        let home = std::env::var("HOME").unwrap_or_default();
        let config_path = format!("{}/.aws/config", home);
        let credentials_path = format!("{}/.aws/credentials", home);

        // Parse config file
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            let mut current_profile: Option<String> = None;
            let mut current_region: Option<String> = None;
            let mut current_role: Option<String> = None;
            let mut current_source: Option<String> = None;

            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    if let Some(name) = current_profile.take() {
                        profiles.push(Profile {
                            name,
                            region: current_region.take(),
                            account: None,
                            role_arn: current_role.take(),
                            source_profile: current_source.take(),
                        });
                    }
                    let profile_name = line
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .trim_start_matches("profile ")
                        .to_string();
                    current_profile = Some(profile_name);
                } else if let Some(key_value) = line.split_once('=') {
                    let key = key_value.0.trim();
                    let value = key_value.1.trim().to_string();
                    match key {
                        "region" => current_region = Some(value),
                        "role_arn" => current_role = Some(value),
                        "source_profile" => current_source = Some(value),
                        _ => {}
                    }
                }
            }
            if let Some(name) = current_profile {
                profiles.push(Profile {
                    name,
                    region: current_region,
                    account: None,
                    role_arn: current_role,
                    source_profile: current_source,
                });
            }
        }

        // Parse credentials file for additional profiles
        if let Ok(content) = std::fs::read_to_string(&credentials_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    let profile_name = line
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .to_string();
                    if !profiles.iter().any(|p| p.name == profile_name) {
                        profiles.push(Profile {
                            name: profile_name,
                            region: None,
                            account: None,
                            role_arn: None,
                            source_profile: None,
                        });
                    }
                }
            }
        }

        profiles
    }
}

pub fn render_profile_picker(
    frame: &mut ratatui::Frame,
    app: &crate::app::App,
    area: ratatui::prelude::Rect,
    centered_rect: fn(u16, u16, ratatui::prelude::Rect) -> ratatui::prelude::Rect,
) {
    let popup_area = centered_rect(80, 70, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    let cursor = "█";
    let filter = Paragraph::new(Line::from(vec![
        Span::raw(&app.profile_filter),
        Span::styled(cursor, Style::default().fg(Color::Green)),
    ]))
    .block(
        Block::default()
            .title(" 🔍 ")
            .borders(Borders::ALL)
            .border_style(active_border()),
    )
    .style(Style::default());

    frame.render_widget(Clear, popup_area);
    frame.render_widget(filter, chunks[0]);

    struct ProfileNameColumn;
    impl TableColumn<Profile> for ProfileNameColumn {
        fn name(&self) -> &str {
            "Profile"
        }
        fn width(&self) -> u16 {
            25
        }
        fn render(&self, item: &Profile) -> (String, Style) {
            (item.name.clone(), Style::default())
        }
    }

    struct ProfileAccountColumn;
    impl TableColumn<Profile> for ProfileAccountColumn {
        fn name(&self) -> &str {
            "Account"
        }
        fn width(&self) -> u16 {
            15
        }
        fn render(&self, item: &Profile) -> (String, Style) {
            (item.account.clone().unwrap_or_default(), Style::default())
        }
    }

    struct ProfileRegionColumn;
    impl TableColumn<Profile> for ProfileRegionColumn {
        fn name(&self) -> &str {
            "Region"
        }
        fn width(&self) -> u16 {
            15
        }
        fn render(&self, item: &Profile) -> (String, Style) {
            (item.region.clone().unwrap_or_default(), Style::default())
        }
    }

    struct ProfileRoleColumn;
    impl TableColumn<Profile> for ProfileRoleColumn {
        fn name(&self) -> &str {
            "Role/User"
        }
        fn width(&self) -> u16 {
            30
        }
        fn render(&self, item: &Profile) -> (String, Style) {
            if let Some(ref role) = item.role_arn {
                if role.contains(":role/") {
                    let role_name = role.split('/').next_back().unwrap_or(role);
                    (format!("role/{}", role_name), Style::default())
                } else if role.contains(":user/") {
                    let user_name = role.split('/').next_back().unwrap_or(role);
                    (format!("user/{}", user_name), Style::default())
                } else {
                    (role.clone(), Style::default())
                }
            } else {
                (String::new(), Style::default())
            }
        }
    }

    struct ProfileSourceColumn;
    impl TableColumn<Profile> for ProfileSourceColumn {
        fn name(&self) -> &str {
            "Source"
        }
        fn width(&self) -> u16 {
            20
        }
        fn render(&self, item: &Profile) -> (String, Style) {
            (
                item.source_profile.clone().unwrap_or_default(),
                Style::default(),
            )
        }
    }

    let columns: Vec<Box<dyn TableColumn<Profile>>> = vec![
        Box::new(ProfileNameColumn),
        Box::new(ProfileAccountColumn),
        Box::new(ProfileRegionColumn),
        Box::new(ProfileRoleColumn),
        Box::new(ProfileSourceColumn),
    ];

    let filtered = app.get_filtered_profiles();
    let config = TableConfig {
        items: filtered,
        selected_index: app.profile_picker_selected,
        expanded_index: None,
        columns: &columns,
        sort_column: "Profile",
        sort_direction: SortDirection::Asc,
        title: " Profiles (^R to fetch accounts) ".to_string(),
        area: chunks[1],
        get_expanded_content: None,
        is_active: true,
    };

    render_table(frame, config);
}

pub fn filter_profiles<'a>(profiles: &'a [Profile], filter: &str) -> Vec<&'a Profile> {
    let mut filtered: Vec<&Profile> = if filter.is_empty() {
        profiles.iter().collect()
    } else {
        let filter_lower = filter.to_lowercase();
        profiles
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&filter_lower)
                    || p.region
                        .as_ref()
                        .is_some_and(|r| r.to_lowercase().contains(&filter_lower))
                    || p.account
                        .as_ref()
                        .is_some_and(|a| a.to_lowercase().contains(&filter_lower))
                    || p.role_arn
                        .as_ref()
                        .is_some_and(|r| r.to_lowercase().contains(&filter_lower))
                    || p.source_profile
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(&filter_lower))
            })
            .collect()
    };
    filtered.sort_by(|a, b| a.name.cmp(&b.name));
    filtered
}
