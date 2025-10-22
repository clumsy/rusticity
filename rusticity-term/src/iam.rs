use crate::common::UTC_TIMESTAMP_WIDTH;
use serde::{Deserialize, Serialize};

pub fn format_arn(account_id: &str, resource_type: &str, resource_name: &str) -> String {
    format!(
        "arn:aws:iam::{}:{}/{}",
        account_id, resource_type, resource_name
    )
}

pub fn console_url_users(_region: &str) -> String {
    "https://console.aws.amazon.com/iam/home#/users".to_string()
}

pub fn console_url_user_detail(region: &str, user_name: &str, section: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/iam/home?region={}#/users/details/{}?section={}",
        region, region, user_name, section
    )
}

pub fn console_url_roles(_region: &str) -> String {
    "https://console.aws.amazon.com/iam/home#/roles".to_string()
}

pub fn console_url_role_detail(region: &str, role_name: &str, section: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/iam/home?region={}#/roles/details/{}?section={}",
        region, region, role_name, section
    )
}

pub fn console_url_role_policy(region: &str, role_name: &str, policy_name: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/iam/home?region={}#/roles/details/{}/editPolicy/{}?step=addPermissions",
        region, region, role_name, policy_name
    )
}

pub fn console_url_groups(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/iam/home?region={}#/groups",
        region, region
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamUser {
    pub user_name: String,
    pub path: String,
    pub groups: String,
    pub last_activity: String,
    pub mfa: String,
    pub password_age: String,
    pub console_last_sign_in: String,
    pub access_key_id: String,
    pub active_key_age: String,
    pub access_key_last_used: String,
    pub arn: String,
    pub creation_time: String,
    pub console_access: String,
    pub signing_certs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamRole {
    pub role_name: String,
    pub path: String,
    pub trusted_entities: String,
    pub last_activity: String,
    pub arn: String,
    pub creation_time: String,
    pub description: String,
    pub max_session_duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamGroup {
    pub group_name: String,
    pub path: String,
    pub users: String,
    pub permissions: String,
    pub creation_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub policy_name: String,
    pub policy_type: String,
    pub attached_via: String,
    pub attached_entities: String,
    pub description: String,
    pub creation_time: String,
    pub edited_time: String,
    pub policy_arn: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoleTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct UserTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGroup {
    pub group_name: String,
    pub attached_policies: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupUser {
    pub user_name: String,
    pub groups: String,
    pub last_activity: String,
    pub creation_time: String,
}

#[derive(Debug, Clone)]
pub struct LastAccessedService {
    pub service: String,
    pub policies_granting: String,
    pub last_accessed: String,
}

#[derive(Debug, Clone, Copy)]
pub enum UserColumn {
    UserName,
    Path,
    Groups,
    LastActivity,
    Mfa,
    PasswordAge,
    ConsoleLastSignIn,
    AccessKeyId,
    ActiveKeyAge,
    AccessKeyLastUsed,
    Arn,
    CreationTime,
    ConsoleAccess,
    SigningCerts,
}

#[derive(Debug, Clone, Copy)]
pub enum GroupColumn {
    GroupName,
    Path,
    Users,
    Permissions,
    CreationTime,
}

#[derive(Debug, Clone, Copy)]
pub enum RoleColumn {
    RoleName,
    Path,
    TrustedEntities,
    LastActivity,
    Arn,
    CreationTime,
    Description,
    MaxSessionDuration,
}

#[derive(Debug, Clone, Copy)]
pub enum GroupUserColumn {
    UserName,
    Groups,
    LastActivity,
    CreationTime,
}

impl UserColumn {
    pub fn to_column(self) -> Box<dyn for<'a> crate::ui::table::Column<&'a IamUser>> {
        use crate::ui::table::Column as TableColumn;
        use ratatui::style::Style;

        struct UserCol(UserColumn);
        impl<'a> TableColumn<&'a IamUser> for UserCol {
            fn name(&self) -> &str {
                match self.0 {
                    UserColumn::UserName => "User name",
                    UserColumn::Path => "Path",
                    UserColumn::Groups => "Groups",
                    UserColumn::LastActivity => "Last activity",
                    UserColumn::Mfa => "MFA",
                    UserColumn::PasswordAge => "Password age",
                    UserColumn::ConsoleLastSignIn => "Console last sign-in",
                    UserColumn::AccessKeyId => "Access key ID",
                    UserColumn::ActiveKeyAge => "Active key age",
                    UserColumn::AccessKeyLastUsed => "Access key last used",
                    UserColumn::Arn => "ARN",
                    UserColumn::CreationTime => "Creation time",
                    UserColumn::ConsoleAccess => "Console access",
                    UserColumn::SigningCerts => "Signing certs",
                }
            }
            fn width(&self) -> u16 {
                let custom = match self.0 {
                    UserColumn::UserName => 20,
                    UserColumn::Path => 15,
                    UserColumn::Groups => 20,
                    UserColumn::LastActivity => 20,
                    UserColumn::Mfa => 10,
                    UserColumn::PasswordAge => 15,
                    UserColumn::ConsoleLastSignIn => 25,
                    UserColumn::AccessKeyId => 25,
                    UserColumn::ActiveKeyAge => 18,
                    UserColumn::AccessKeyLastUsed => UTC_TIMESTAMP_WIDTH as usize,
                    UserColumn::Arn => 50,
                    UserColumn::CreationTime => 30,
                    UserColumn::ConsoleAccess => 15,
                    UserColumn::SigningCerts => 15,
                };
                self.name().len().max(custom) as u16
            }
            fn render(&self, item: &&'a IamUser) -> (String, Style) {
                let value = match self.0 {
                    UserColumn::UserName => {
                        return (item.user_name.clone(), Style::default());
                    }
                    UserColumn::Path => &item.path,
                    UserColumn::Groups => &item.groups,
                    UserColumn::LastActivity => &item.last_activity,
                    UserColumn::Mfa => &item.mfa,
                    UserColumn::PasswordAge => &item.password_age,
                    UserColumn::ConsoleLastSignIn => &item.console_last_sign_in,
                    UserColumn::AccessKeyId => &item.access_key_id,
                    UserColumn::ActiveKeyAge => &item.active_key_age,
                    UserColumn::AccessKeyLastUsed => &item.access_key_last_used,
                    UserColumn::Arn => &item.arn,
                    UserColumn::CreationTime => &item.creation_time,
                    UserColumn::ConsoleAccess => &item.console_access,
                    UserColumn::SigningCerts => &item.signing_certs,
                };
                (value.clone(), Style::default())
            }
        }
        Box::new(UserCol(self))
    }
}

impl GroupColumn {
    pub fn to_column(self) -> Box<dyn crate::ui::table::Column<IamGroup>> {
        use crate::ui::table::Column as TableColumn;
        use ratatui::style::Style;

        struct GroupCol(GroupColumn);
        impl TableColumn<IamGroup> for GroupCol {
            fn name(&self) -> &str {
                match self.0 {
                    GroupColumn::GroupName => "Group name",
                    GroupColumn::Path => "Path",
                    GroupColumn::Users => "Users",
                    GroupColumn::Permissions => "Permissions",
                    GroupColumn::CreationTime => "Creation time",
                }
            }
            fn width(&self) -> u16 {
                let custom = match self.0 {
                    GroupColumn::GroupName => 20,
                    GroupColumn::Path => 15,
                    GroupColumn::Users => 10,
                    GroupColumn::Permissions => 20,
                    GroupColumn::CreationTime => 30,
                };
                self.name().len().max(custom) as u16
            }
            fn render(&self, item: &IamGroup) -> (String, Style) {
                use ratatui::style::Color;
                match self.0 {
                    GroupColumn::GroupName => (item.group_name.clone(), Style::default()),
                    GroupColumn::Permissions if item.permissions == "Defined" => (
                        format!("✅ {}", item.permissions),
                        Style::default().fg(Color::Green),
                    ),
                    GroupColumn::Path => (item.path.clone(), Style::default()),
                    GroupColumn::Users => (item.users.clone(), Style::default()),
                    GroupColumn::Permissions => (item.permissions.clone(), Style::default()),
                    GroupColumn::CreationTime => (item.creation_time.clone(), Style::default()),
                }
            }
        }
        Box::new(GroupCol(self))
    }
}

impl RoleColumn {
    pub fn to_column(self) -> Box<dyn crate::ui::table::Column<IamRole>> {
        use crate::ui::table::Column as TableColumn;
        use ratatui::style::Style;

        struct RoleCol(RoleColumn);
        impl TableColumn<IamRole> for RoleCol {
            fn name(&self) -> &str {
                match self.0 {
                    RoleColumn::RoleName => "Role name",
                    RoleColumn::Path => "Path",
                    RoleColumn::TrustedEntities => "Trusted entities",
                    RoleColumn::LastActivity => "Last activity",
                    RoleColumn::Arn => "ARN",
                    RoleColumn::CreationTime => "Creation time",
                    RoleColumn::Description => "Description",
                    RoleColumn::MaxSessionDuration => "Max CLI/API session",
                }
            }
            fn width(&self) -> u16 {
                let custom = match self.0 {
                    RoleColumn::RoleName => 30,
                    RoleColumn::Path => 15,
                    RoleColumn::TrustedEntities => 30,
                    RoleColumn::LastActivity => 20,
                    RoleColumn::Arn => 50,
                    RoleColumn::CreationTime => 30,
                    RoleColumn::Description => 40,
                    RoleColumn::MaxSessionDuration => 22,
                };
                self.name().len().max(custom) as u16
            }
            fn render(&self, item: &IamRole) -> (String, Style) {
                let value = match self.0 {
                    RoleColumn::RoleName => {
                        return (item.role_name.clone(), Style::default());
                    }
                    RoleColumn::Path => &item.path,
                    RoleColumn::TrustedEntities => &item.trusted_entities,
                    RoleColumn::LastActivity => &item.last_activity,
                    RoleColumn::Arn => &item.arn,
                    RoleColumn::CreationTime => &item.creation_time,
                    RoleColumn::Description => &item.description,
                    RoleColumn::MaxSessionDuration => &item.max_session_duration,
                };
                (value.clone(), Style::default())
            }
        }
        Box::new(RoleCol(self))
    }
}

impl GroupUserColumn {
    pub fn to_column(self) -> Box<dyn crate::ui::table::Column<GroupUser>> {
        use crate::ui::table::Column as TableColumn;
        use ratatui::style::Style;

        struct GroupUserCol(GroupUserColumn);
        impl TableColumn<GroupUser> for GroupUserCol {
            fn name(&self) -> &str {
                match self.0 {
                    GroupUserColumn::UserName => "User name",
                    GroupUserColumn::Groups => "Groups",
                    GroupUserColumn::LastActivity => "Last activity",
                    GroupUserColumn::CreationTime => "Creation time",
                }
            }
            fn width(&self) -> u16 {
                let custom = match self.0 {
                    GroupUserColumn::UserName => 20,
                    GroupUserColumn::Groups => 20,
                    GroupUserColumn::LastActivity => 20,
                    GroupUserColumn::CreationTime => 30,
                };
                self.name().len().max(custom) as u16
            }
            fn render(&self, item: &GroupUser) -> (String, Style) {
                match self.0 {
                    GroupUserColumn::UserName => (item.user_name.clone(), Style::default()),
                    GroupUserColumn::Groups => (item.groups.clone(), Style::default()),
                    GroupUserColumn::LastActivity => (item.last_activity.clone(), Style::default()),
                    GroupUserColumn::CreationTime => (item.creation_time.clone(), Style::default()),
                }
            }
        }
        Box::new(GroupUserCol(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::CyclicEnum;
    use crate::ui::iam::{GroupTab, State, UserTab};

    #[test]
    fn test_user_group_creation() {
        let group = UserGroup {
            group_name: "Developers".to_string(),
            attached_policies: "AmazonS3ReadOnlyAccess, AmazonEC2ReadOnlyAccess".to_string(),
        };
        assert_eq!(group.group_name, "Developers");
        assert_eq!(
            group.attached_policies,
            "AmazonS3ReadOnlyAccess, AmazonEC2ReadOnlyAccess"
        );
    }

    #[test]
    fn test_iam_state_user_group_memberships_initialization() {
        let state = State::new();
        assert_eq!(state.user_group_memberships.items.len(), 0);
        assert_eq!(state.user_group_memberships.selected, 0);
        assert_eq!(state.user_group_memberships.filter, "");
    }

    #[test]
    fn test_user_tab_groups() {
        let tab = UserTab::Permissions;
        assert_eq!(tab.next(), UserTab::Groups);
        assert_eq!(UserTab::Groups.name(), "Groups");
    }

    #[test]
    fn test_group_tab_navigation() {
        let tab = GroupTab::Users;
        assert_eq!(tab.next(), GroupTab::Permissions);
        assert_eq!(tab.next().next(), GroupTab::AccessAdvisor);
        assert_eq!(tab.next().next().next(), GroupTab::Users);
    }

    #[test]
    fn test_group_tab_names() {
        assert_eq!(GroupTab::Users.name(), "Users");
        assert_eq!(GroupTab::Permissions.name(), "Permissions");
        assert_eq!(GroupTab::AccessAdvisor.name(), "Access Advisor");
    }

    #[test]
    fn test_iam_state_group_tab_initialization() {
        let state = State::new();
        assert_eq!(state.group_tab, GroupTab::Users);
    }
}
