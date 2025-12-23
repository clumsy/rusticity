use crate::common::t;
use crate::common::{format_duration_seconds, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in UserColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
    for col in GroupColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
    for col in RoleColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

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
    pub max_session_duration: Option<i32>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl UserColumn {
    const ID_USER_NAME: &'static str = "column.iam.user.user_name";
    const ID_PATH: &'static str = "column.iam.user.path";
    const ID_GROUPS: &'static str = "column.iam.user.groups";
    const ID_LAST_ACTIVITY: &'static str = "column.iam.user.last_activity";
    const ID_MFA: &'static str = "column.iam.user.mfa";
    const ID_PASSWORD_AGE: &'static str = "column.iam.user.password_age";
    const ID_CONSOLE_LAST_SIGN_IN: &'static str = "column.iam.user.console_last_sign_in";
    const ID_ACCESS_KEY_ID: &'static str = "column.iam.user.access_key_id";
    const ID_ACTIVE_KEY_AGE: &'static str = "column.iam.user.active_key_age";
    const ID_ACCESS_KEY_LAST_USED: &'static str = "column.iam.user.access_key_last_used";
    const ID_ARN: &'static str = "column.iam.user.arn";
    const ID_CREATION_TIME: &'static str = "column.iam.user.creation_time";
    const ID_CONSOLE_ACCESS: &'static str = "column.iam.user.console_access";
    const ID_SIGNING_CERTS: &'static str = "column.iam.user.signing_certs";

    pub const fn id(&self) -> &'static str {
        match self {
            Self::UserName => Self::ID_USER_NAME,
            Self::Path => Self::ID_PATH,
            Self::Groups => Self::ID_GROUPS,
            Self::LastActivity => Self::ID_LAST_ACTIVITY,
            Self::Mfa => Self::ID_MFA,
            Self::PasswordAge => Self::ID_PASSWORD_AGE,
            Self::ConsoleLastSignIn => Self::ID_CONSOLE_LAST_SIGN_IN,
            Self::AccessKeyId => Self::ID_ACCESS_KEY_ID,
            Self::ActiveKeyAge => Self::ID_ACTIVE_KEY_AGE,
            Self::AccessKeyLastUsed => Self::ID_ACCESS_KEY_LAST_USED,
            Self::Arn => Self::ID_ARN,
            Self::CreationTime => Self::ID_CREATION_TIME,
            Self::ConsoleAccess => Self::ID_CONSOLE_ACCESS,
            Self::SigningCerts => Self::ID_SIGNING_CERTS,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Self::UserName => "User name",
            Self::Path => "Path",
            Self::Groups => "Groups",
            Self::LastActivity => "Last activity",
            Self::Mfa => "MFA",
            Self::PasswordAge => "Password age",
            Self::ConsoleLastSignIn => "Console last sign-in",
            Self::AccessKeyId => "Access key ID",
            Self::ActiveKeyAge => "Active key age",
            Self::AccessKeyLastUsed => "Access key last used",
            Self::Arn => "ARN",
            Self::CreationTime => "Creation time",
            Self::ConsoleAccess => "Console access",
            Self::SigningCerts => "Signing certificates",
        }
    }

    pub fn name(&self) -> String {
        let key = self.id();
        let translated = t(key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_USER_NAME => Some(Self::UserName),
            Self::ID_PATH => Some(Self::Path),
            Self::ID_GROUPS => Some(Self::Groups),
            Self::ID_LAST_ACTIVITY => Some(Self::LastActivity),
            Self::ID_MFA => Some(Self::Mfa),
            Self::ID_PASSWORD_AGE => Some(Self::PasswordAge),
            Self::ID_CONSOLE_LAST_SIGN_IN => Some(Self::ConsoleLastSignIn),
            Self::ID_ACCESS_KEY_ID => Some(Self::AccessKeyId),
            Self::ID_ACTIVE_KEY_AGE => Some(Self::ActiveKeyAge),
            Self::ID_ACCESS_KEY_LAST_USED => Some(Self::AccessKeyLastUsed),
            Self::ID_ARN => Some(Self::Arn),
            Self::ID_CREATION_TIME => Some(Self::CreationTime),
            Self::ID_CONSOLE_ACCESS => Some(Self::ConsoleAccess),
            Self::ID_SIGNING_CERTS => Some(Self::SigningCerts),
            _ => None,
        }
    }

    pub fn all() -> [UserColumn; 14] {
        [
            Self::UserName,
            Self::Path,
            Self::Groups,
            Self::LastActivity,
            Self::Mfa,
            Self::PasswordAge,
            Self::ConsoleLastSignIn,
            Self::AccessKeyId,
            Self::ActiveKeyAge,
            Self::AccessKeyLastUsed,
            Self::Arn,
            Self::CreationTime,
            Self::ConsoleAccess,
            Self::SigningCerts,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn visible() -> Vec<ColumnId> {
        vec![
            Self::UserName.id(),
            Self::Path.id(),
            Self::Groups.id(),
            Self::LastActivity.id(),
            Self::Mfa.id(),
            Self::PasswordAge.id(),
            Self::ConsoleLastSignIn.id(),
            Self::AccessKeyId.id(),
            Self::ActiveKeyAge.id(),
            Self::AccessKeyLastUsed.id(),
            Self::Arn.id(),
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GroupColumn {
    GroupName,
    Path,
    Users,
    Permissions,
    CreationTime,
}

impl GroupColumn {
    pub fn all() -> [GroupColumn; 5] {
        [
            Self::GroupName,
            Self::Path,
            Self::Users,
            Self::Permissions,
            Self::CreationTime,
        ]
    }
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

impl RoleColumn {
    const ID_ROLE_NAME: &'static str = "column.iam.role.role_name";
    const ID_PATH: &'static str = "column.iam.role.path";
    const ID_TRUSTED_ENTITIES: &'static str = "column.iam.role.trusted_entities";
    const ID_LAST_ACTIVITY: &'static str = "column.iam.role.last_activity";
    const ID_ARN: &'static str = "column.iam.role.arn";
    const ID_CREATION_TIME: &'static str = "column.iam.role.creation_time";
    const ID_DESCRIPTION: &'static str = "column.iam.role.description";
    const ID_MAX_SESSION_DURATION: &'static str = "column.iam.role.max_session_duration";

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_ROLE_NAME => Some(Self::RoleName),
            Self::ID_PATH => Some(Self::Path),
            Self::ID_TRUSTED_ENTITIES => Some(Self::TrustedEntities),
            Self::ID_LAST_ACTIVITY => Some(Self::LastActivity),
            Self::ID_ARN => Some(Self::Arn),
            Self::ID_CREATION_TIME => Some(Self::CreationTime),
            Self::ID_DESCRIPTION => Some(Self::Description),
            Self::ID_MAX_SESSION_DURATION => Some(Self::MaxSessionDuration),
            _ => None,
        }
    }

    pub const fn id(&self) -> ColumnId {
        match self {
            Self::RoleName => Self::ID_ROLE_NAME,
            Self::Path => Self::ID_PATH,
            Self::TrustedEntities => Self::ID_TRUSTED_ENTITIES,
            Self::LastActivity => Self::ID_LAST_ACTIVITY,
            Self::Arn => Self::ID_ARN,
            Self::CreationTime => Self::ID_CREATION_TIME,
            Self::Description => Self::ID_DESCRIPTION,
            Self::MaxSessionDuration => Self::ID_MAX_SESSION_DURATION,
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Self::RoleName => "Role name",
            Self::Path => "Path",
            Self::TrustedEntities => "Trusted entities",
            Self::LastActivity => "Last activity",
            Self::Arn => "ARN",
            Self::CreationTime => "Creation time",
            Self::Description => "Description",
            Self::MaxSessionDuration => "Max session duration",
        }
    }

    pub fn name(&self) -> String {
        let key = self.id();
        let translated = t(key);
        if translated == key {
            self.default_name().to_string()
        } else {
            translated
        }
    }

    pub fn all() -> [RoleColumn; 8] {
        [
            Self::RoleName,
            Self::Path,
            Self::TrustedEntities,
            Self::LastActivity,
            Self::Arn,
            Self::CreationTime,
            Self::Description,
            Self::MaxSessionDuration,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn visible() -> Vec<ColumnId> {
        vec![
            Self::RoleName.id(),
            Self::TrustedEntities.id(),
            Self::CreationTime.id(),
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GroupUserColumn {
    UserName,
    Groups,
    LastActivity,
    CreationTime,
}

#[derive(Debug, Clone, Copy)]
pub enum PolicyColumn {
    PolicyName,
    Type,
    AttachedVia,
    AttachedEntities,
    Description,
    CreationTime,
    EditedTime,
}

#[derive(Debug, Clone, Copy)]
pub enum TagColumn {
    Key,
    Value,
}

#[derive(Debug, Clone, Copy)]
pub enum UserGroupColumn {
    GroupName,
    AttachedPolicies,
}

#[derive(Debug, Clone, Copy)]
pub enum LastAccessedServiceColumn {
    Service,
    PoliciesGranting,
    LastAccessed,
}

impl<'a> Column<&'a IamUser> for UserColumn {
    fn id(&self) -> &'static str {
        UserColumn::id(self)
    }

    fn default_name(&self) -> &'static str {
        UserColumn::default_name(self)
    }
            Self::Path => "Path",
            Self::Groups => "Groups",
            Self::LastActivity => "Last activity",
            Self::Mfa => "MFA",
            Self::PasswordAge => "Password age",
            Self::ConsoleLastSignIn => "Console last sign-in",
            Self::AccessKeyId => "Access key ID",
            Self::ActiveKeyAge => "Active key age",
            Self::AccessKeyLastUsed => "Access key last used",
            Self::Arn => "ARN",
            Self::CreationTime => "Creation time",
            Self::ConsoleAccess => "Console access",
            Self::SigningCerts => "Signing certificates",
        }
    }

    fn name(&self) -> &str {
        let key = self.id();
        let translated = t(key);
        if translated == key {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16 {
        let custom = match self {
            Self::UserName => 20,
            Self::Path => 15,
            Self::Groups => 20,
            Self::LastActivity => 20,
            Self::Mfa => 10,
            Self::PasswordAge => 15,
            Self::ConsoleLastSignIn => 25,
            Self::AccessKeyId => 25,
            Self::ActiveKeyAge => 18,
            Self::AccessKeyLastUsed => UTC_TIMESTAMP_WIDTH as usize,
            Self::Arn => 50,
            Self::CreationTime => 30,
            Self::ConsoleAccess => 15,
            Self::SigningCerts => 15,
        };
        self.name().len().max(custom) as u16
    }

    fn render(&self, item: &&'a IamUser) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::UserName => item.user_name.clone(),
            Self::Path => item.path.clone(),
            Self::Groups => item.groups.clone(),
            Self::LastActivity => item.last_activity.clone(),
            Self::Mfa => item.mfa.clone(),
            Self::PasswordAge => item.password_age.clone(),
            Self::ConsoleLastSignIn => item.console_last_sign_in.clone(),
            Self::AccessKeyId => item.access_key_id.clone(),
            Self::ActiveKeyAge => item.active_key_age.clone(),
            Self::AccessKeyLastUsed => item.access_key_last_used.clone(),
            Self::Arn => item.arn.clone(),
            Self::CreationTime => item.creation_time.clone(),
            Self::ConsoleAccess => item.console_access.clone(),
            Self::SigningCerts => item.signing_certs.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<IamGroup> for GroupColumn {
    fn id(&self) -> &'static str {
        match self {
            Self::GroupName => "column.iam.group.group_name",
            Self::Path => "column.iam.group.path",
            Self::Users => "column.iam.group.users",
            Self::Permissions => "column.iam.group.permissions",
            Self::CreationTime => "column.iam.group.creation_time",
        }
    }

    fn default_name(&self) -> &'static str {
        match self {
            Self::GroupName => "Group name",
            Self::Path => "Path",
            Self::Users => "Users",
            Self::Permissions => "Permissions",
            Self::CreationTime => "Creation time",
        }
    }

    fn width(&self) -> u16 {
        let custom = match self {
            Self::GroupName => 20,
            Self::Path => 15,
            Self::Users => 10,
            Self::Permissions => 20,
            Self::CreationTime => 30,
        };
        self.name().len().max(custom) as u16
    }

    fn render(&self, item: &IamGroup) -> (String, ratatui::style::Style) {
        use ratatui::style::{Color, Style};
        match self {
            Self::GroupName => (item.group_name.clone(), Style::default()),
            Self::Permissions if item.permissions == "Defined" => (
                format!("✅ {}", item.permissions),
                Style::default().fg(Color::Green),
            ),
            Self::Path => (item.path.clone(), Style::default()),
            Self::Users => (item.users.clone(), Style::default()),
            Self::Permissions => (item.permissions.clone(), Style::default()),
            Self::CreationTime => (item.creation_time.clone(), Style::default()),
        }
    }
}

impl Column<IamRole> for RoleColumn {
    fn id(&self) -> &'static str {
        match self {
            Self::RoleName => "column.iam.role.role_name",
            Self::Path => "column.iam.role.path",
            Self::TrustedEntities => "column.iam.role.trusted_entities",
            Self::LastActivity => "column.iam.role.last_activity",
            Self::Arn => "column.iam.role.arn",
            Self::CreationTime => "column.iam.role.creation_time",
            Self::Description => "column.iam.role.description",
            Self::MaxSessionDuration => "column.iam.role.max_session_duration",
        }
    }

    fn default_name(&self) -> &'static str {
        match self {
            Self::RoleName => "Role name",
            Self::Path => "Path",
            Self::TrustedEntities => "Trusted entities",
            Self::LastActivity => "Last activity",
            Self::Arn => "ARN",
            Self::CreationTime => "Creation time",
            Self::Description => "Description",
            Self::MaxSessionDuration => "Max CLI/API session",
        }
    }

    fn width(&self) -> u16 {
        let custom = match self {
            Self::RoleName => 30,
            Self::Path => 15,
            Self::TrustedEntities => 30,
            Self::LastActivity => 20,
            Self::Arn => 50,
            Self::CreationTime => 30,
            Self::Description => 40,
            Self::MaxSessionDuration => 22,
        };
        self.name().len().max(custom) as u16
    }

    fn render(&self, item: &IamRole) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::RoleName => item.role_name.clone(),
            Self::Path => item.path.clone(),
            Self::TrustedEntities => item.trusted_entities.clone(),
            Self::LastActivity => item.last_activity.clone(),
            Self::Arn => item.arn.clone(),
            Self::CreationTime => item.creation_time.clone(),
            Self::Description => item.description.clone(),
            Self::MaxSessionDuration => item
                .max_session_duration
                .map(format_duration_seconds)
                .unwrap_or_default(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<GroupUser> for GroupUserColumn {
    fn name(&self) -> &str {
        match self {
            Self::UserName => "User name",
            Self::Groups => "Groups",
            Self::LastActivity => "Last activity",
            Self::CreationTime => "Creation time",
        }
    }

    fn width(&self) -> u16 {
        let custom = match self {
            Self::UserName => 20,
            Self::Groups => 20,
            Self::LastActivity => 20,
            Self::CreationTime => 30,
        };
        self.name().len().max(custom) as u16
    }

    fn render(&self, item: &GroupUser) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::UserName => item.user_name.clone(),
            Self::Groups => item.groups.clone(),
            Self::LastActivity => item.last_activity.clone(),
            Self::CreationTime => item.creation_time.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<Policy> for PolicyColumn {
    fn name(&self) -> &str {
        match self {
            Self::PolicyName => "Policy name",
            Self::Type => "Type",
            Self::AttachedVia => "Attached via",
            Self::AttachedEntities => "Attached entities",
            Self::Description => "Description",
            Self::CreationTime => "Creation time",
            Self::EditedTime => "Edited time",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::PolicyName => 30,
            Self::Type => 15,
            Self::AttachedVia => 20,
            Self::AttachedEntities => 20,
            Self::Description => 40,
            Self::CreationTime => 30,
            Self::EditedTime => 30,
        }
    }

    fn render(&self, item: &Policy) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::PolicyName => item.policy_name.clone(),
            Self::Type => item.policy_type.clone(),
            Self::AttachedVia => item.attached_via.clone(),
            Self::AttachedEntities => item.attached_entities.clone(),
            Self::Description => item.description.clone(),
            Self::CreationTime => item.creation_time.clone(),
            Self::EditedTime => item.edited_time.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<RoleTag> for TagColumn {
    fn name(&self) -> &str {
        match self {
            Self::Key => "Key",
            Self::Value => "Value",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::Key => 30,
            Self::Value => 70,
        }
    }

    fn render(&self, item: &RoleTag) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::Key => item.key.clone(),
            Self::Value => item.value.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<UserTag> for TagColumn {
    fn name(&self) -> &str {
        match self {
            Self::Key => "Key",
            Self::Value => "Value",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::Key => 30,
            Self::Value => 70,
        }
    }

    fn render(&self, item: &UserTag) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::Key => item.key.clone(),
            Self::Value => item.value.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<UserGroup> for UserGroupColumn {
    fn name(&self) -> &str {
        match self {
            Self::GroupName => "Group name",
            Self::AttachedPolicies => "Attached policies",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::GroupName => 40,
            Self::AttachedPolicies => 60,
        }
    }

    fn render(&self, item: &UserGroup) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::GroupName => item.group_name.clone(),
            Self::AttachedPolicies => item.attached_policies.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

impl Column<LastAccessedService> for LastAccessedServiceColumn {
    fn name(&self) -> &str {
        match self {
            Self::Service => "Service",
            Self::PoliciesGranting => "Policies granting permissions",
            Self::LastAccessed => "Last accessed",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::Service => 30,
            Self::PoliciesGranting => 40,
            Self::LastAccessed => 30,
        }
    }

    fn render(&self, item: &LastAccessedService) -> (String, ratatui::style::Style) {
        let text = match self {
            Self::Service => item.service.clone(),
            Self::PoliciesGranting => item.policies_granting.clone(),
            Self::LastAccessed => item.last_accessed.clone(),
        };
        (text, ratatui::style::Style::default())
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
