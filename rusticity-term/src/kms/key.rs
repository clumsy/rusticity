use crate::common::{format_iso_timestamp, translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
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
pub struct Key {
    pub key_id: String,
    pub key_arn: String,
    pub alias: String,
    pub description: String,
    pub key_state: String,
    pub key_usage: String,
    pub key_spec: String,
    pub key_manager: String,
    pub creation_date: String,
    pub expiration_date: String,
    pub deletion_date: String,
    pub custom_key_store_id: String,
    pub origin: String,
    pub enabled: bool,
    pub multi_region: bool,
}

/// All possible columns across both tabs.
/// AWS managed keys: Alias(locked), KeyId, KeyState, CreationDate,
///   ExpirationDate, DeletionDate, Origin, CustomKeyStoreId
/// Customer managed keys: Alias(locked), KeyId, KeyState, KeyType, KeySpec,
///   KeyUsage, CreationDate, ExpirationDate, DeletionDate, Origin,
///   Regionality, CustomKeyStoreId
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Alias,
    KeyId,
    KeyState,
    KeyType,
    KeySpec,
    KeyUsage,
    CreationDate,
    ExpirationDate,
    DeletionDate,
    Origin,
    Regionality,
    CustomKeyStoreId,
}

impl Column {
    const ID_ALIAS: &'static str = "column.kms.key.alias";
    const ID_KEY_ID: &'static str = "column.kms.key.key_id";
    const ID_KEY_STATE: &'static str = "column.kms.key.key_state";
    const ID_KEY_TYPE: &'static str = "column.kms.key.key_type";
    const ID_KEY_SPEC: &'static str = "column.kms.key.key_spec";
    const ID_KEY_USAGE: &'static str = "column.kms.key.key_usage";
    const ID_CREATION_DATE: &'static str = "column.kms.key.creation_date";
    const ID_EXPIRATION_DATE: &'static str = "column.kms.key.expiration_date";
    const ID_DELETION_DATE: &'static str = "column.kms.key.deletion_date";
    const ID_ORIGIN: &'static str = "column.kms.key.origin";
    const ID_REGIONALITY: &'static str = "column.kms.key.regionality";
    const ID_CUSTOM_KEY_STORE_ID: &'static str = "column.kms.key.custom_key_store_id";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Alias => Self::ID_ALIAS,
            Column::KeyId => Self::ID_KEY_ID,
            Column::KeyState => Self::ID_KEY_STATE,
            Column::KeyType => Self::ID_KEY_TYPE,
            Column::KeySpec => Self::ID_KEY_SPEC,
            Column::KeyUsage => Self::ID_KEY_USAGE,
            Column::CreationDate => Self::ID_CREATION_DATE,
            Column::ExpirationDate => Self::ID_EXPIRATION_DATE,
            Column::DeletionDate => Self::ID_DELETION_DATE,
            Column::Origin => Self::ID_ORIGIN,
            Column::Regionality => Self::ID_REGIONALITY,
            Column::CustomKeyStoreId => Self::ID_CUSTOM_KEY_STORE_ID,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Alias => "Alias",
            Column::KeyId => "Key ID",
            Column::KeyState => "Status",
            Column::KeyType => "Key type",
            Column::KeySpec => "Key spec",
            Column::KeyUsage => "Key usage",
            Column::CreationDate => "Creation date",
            Column::ExpirationDate => "Expiration date",
            Column::DeletionDate => "Schedule deletion on",
            Column::Origin => "Origin",
            Column::Regionality => "Regionality",
            Column::CustomKeyStoreId => "Custom key store ID",
        }
    }

    /// All columns (union of both tabs).
    pub const fn all() -> [Column; 12] {
        [
            Column::Alias,
            Column::KeyId,
            Column::KeyState,
            Column::KeyType,
            Column::KeySpec,
            Column::KeyUsage,
            Column::CreationDate,
            Column::ExpirationDate,
            Column::DeletionDate,
            Column::Origin,
            Column::Regionality,
            Column::CustomKeyStoreId,
        ]
    }

    /// Columns shown in the AWS managed keys tab column selector.
    pub const fn aws_managed_columns() -> [Column; 8] {
        [
            Column::Alias,
            Column::KeyId,
            Column::KeyState,
            Column::CreationDate,
            Column::ExpirationDate,
            Column::DeletionDate,
            Column::Origin,
            Column::CustomKeyStoreId,
        ]
    }

    /// Columns shown in the Customer managed keys tab column selector.
    pub const fn customer_managed_columns() -> [Column; 12] {
        [
            Column::Alias,
            Column::KeyId,
            Column::KeyState,
            Column::KeyType,
            Column::KeySpec,
            Column::KeyUsage,
            Column::CreationDate,
            Column::ExpirationDate,
            Column::DeletionDate,
            Column::Origin,
            Column::Regionality,
            Column::CustomKeyStoreId,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    /// Default visible IDs for AWS managed tab: Alias + Key ID + Status
    pub fn aws_managed_default_visible() -> Vec<ColumnId> {
        vec![Self::ID_ALIAS, Self::ID_KEY_ID, Self::ID_KEY_STATE]
    }

    /// Default visible IDs for Customer managed tab: Alias + Key ID + Status + Key spec + Key usage
    pub fn customer_managed_default_visible() -> Vec<ColumnId> {
        vec![
            Self::ID_ALIAS,
            Self::ID_KEY_ID,
            Self::ID_KEY_STATE,
            Self::ID_KEY_SPEC,
            Self::ID_KEY_USAGE,
        ]
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_ALIAS => Some(Column::Alias),
            Self::ID_KEY_ID => Some(Column::KeyId),
            Self::ID_KEY_STATE => Some(Column::KeyState),
            Self::ID_KEY_TYPE => Some(Column::KeyType),
            Self::ID_KEY_SPEC => Some(Column::KeySpec),
            Self::ID_KEY_USAGE => Some(Column::KeyUsage),
            Self::ID_CREATION_DATE => Some(Column::CreationDate),
            Self::ID_EXPIRATION_DATE => Some(Column::ExpirationDate),
            Self::ID_DELETION_DATE => Some(Column::DeletionDate),
            Self::ID_ORIGIN => Some(Column::Origin),
            Self::ID_REGIONALITY => Some(Column::Regionality),
            Self::ID_CUSTOM_KEY_STORE_ID => Some(Column::CustomKeyStoreId),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<Key> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Alias => 30,
            Column::KeyId => 36,
            Column::KeyState => 20,
            Column::KeyType => 18,
            Column::KeySpec => 20,
            Column::KeyUsage => 20,
            Column::CreationDate => UTC_TIMESTAMP_WIDTH as usize,
            Column::ExpirationDate => UTC_TIMESTAMP_WIDTH as usize,
            Column::DeletionDate => UTC_TIMESTAMP_WIDTH as usize,
            Column::Origin => 14,
            Column::Regionality => 14,
            Column::CustomKeyStoreId => 24,
        }) as u16
    }

    fn render(&self, item: &Key) -> (String, Style) {
        match self {
            Column::Alias => (item.alias.clone(), Style::default()),
            Column::KeyId => (item.key_id.clone(), Style::default()),
            Column::KeyState => {
                let (text, color) = format_key_state(&item.key_state);
                (text.to_string(), Style::default().fg(color))
            }
            Column::KeyType => (
                format_key_type(&item.key_spec, &item.key_usage),
                Style::default(),
            ),
            Column::KeySpec => (format_key_spec(&item.key_spec), Style::default()),
            Column::KeyUsage => (format_key_usage(&item.key_usage), Style::default()),
            Column::CreationDate => (format_iso_timestamp(&item.creation_date), Style::default()),
            Column::ExpirationDate => (
                format_iso_timestamp(&item.expiration_date),
                Style::default(),
            ),
            Column::DeletionDate => (format_iso_timestamp(&item.deletion_date), Style::default()),
            Column::Origin => (format_origin(&item.origin), Style::default()),
            Column::Regionality => (format_regionality(item.multi_region), Style::default()),
            Column::CustomKeyStoreId => (item.custom_key_store_id.clone(), Style::default()),
        }
    }
}

fn format_key_state(state: &str) -> (&'static str, Color) {
    match state {
        "Enabled" => ("✅ Enabled", Color::Green),
        "Disabled" => ("⏸ Disabled", Color::Yellow),
        "PendingDeletion" => ("🗑 Pending deletion", Color::Red),
        "PendingImport" => ("⏳ Pending import", Color::Yellow),
        "PendingReplicaDeletion" => ("🗑 Pending replica deletion", Color::Red),
        "Unavailable" => ("⚠ Unavailable", Color::Red),
        _ => ("Unknown", Color::White),
    }
}

fn format_key_type(spec: &str, usage: &str) -> String {
    // Symmetric vs Asymmetric derived from spec
    match spec {
        "SymmetricDefault" => "Symmetric".to_string(),
        "Hmac224" | "Hmac256" | "Hmac384" | "Hmac512" => "HMAC".to_string(),
        _ => {
            // Asymmetric: RSA, ECC, SM2
            if spec.starts_with("Rsa") || spec.starts_with("Ecc") || spec.starts_with("Sm2") {
                format!("Asymmetric ({})", format_key_usage(usage))
            } else {
                "Asymmetric".to_string()
            }
        }
    }
}

fn format_key_spec(spec: &str) -> String {
    match spec {
        "SymmetricDefault" => "SYMMETRIC_DEFAULT".to_string(),
        "Hmac224" => "HMAC_224".to_string(),
        "Hmac256" => "HMAC_256".to_string(),
        "Hmac384" => "HMAC_384".to_string(),
        "Hmac512" => "HMAC_512".to_string(),
        "Rsa2048" => "RSA_2048".to_string(),
        "Rsa3072" => "RSA_3072".to_string(),
        "Rsa4096" => "RSA_4096".to_string(),
        "EccNistP256" => "ECC_NIST_P256".to_string(),
        "EccNistP384" => "ECC_NIST_P384".to_string(),
        "EccNistP521" => "ECC_NIST_P521".to_string(),
        "EccSecgP256k1" => "ECC_SECG_P256K1".to_string(),
        "Sm2" => "SM2".to_string(),
        s => s.to_string(),
    }
}

fn format_key_usage(usage: &str) -> String {
    match usage {
        "EncryptDecrypt" => "ENCRYPT_DECRYPT".to_string(),
        "SignVerify" => "SIGN_VERIFY".to_string(),
        "GenerateVerifyMac" => "GENERATE_VERIFY_MAC".to_string(),
        "KeyAgreement" => "KEY_AGREEMENT".to_string(),
        s => s.to_string(),
    }
}

fn format_origin(origin: &str) -> String {
    match origin {
        "AwsKms" => "AWS_KMS".to_string(),
        "External" => "EXTERNAL".to_string(),
        "AwsCloudhsm" => "AWS_CLOUDHSM".to_string(),
        "ExternalKeyStore" => "EXTERNAL_KEY_STORE".to_string(),
        s => s.to_string(),
    }
}

fn format_regionality(multi_region: bool) -> String {
    if multi_region {
        "Multi-Region".to_string()
    } else {
        "Single-Region".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.kms.key."),
                "Column ID '{}' should start with 'column.kms.key.'",
                col.id()
            );
        }
    }

    #[test]
    fn test_all_columns_have_default_name() {
        for col in Column::all() {
            assert!(!col.default_name().is_empty());
        }
    }

    #[test]
    fn test_from_id_roundtrip() {
        for col in Column::all() {
            assert_eq!(Column::from_id(col.id()), Some(col));
        }
    }

    #[test]
    fn test_alias_is_first_in_both_tab_columns() {
        assert_eq!(Column::aws_managed_columns()[0], Column::Alias);
        assert_eq!(Column::customer_managed_columns()[0], Column::Alias);
    }

    #[test]
    fn test_aws_managed_defaults() {
        let ids = Column::aws_managed_default_visible();
        assert!(ids.contains(&Column::Alias.id()));
        assert!(ids.contains(&Column::KeyId.id()));
        assert!(ids.contains(&Column::KeyState.id()));
        assert!(
            !ids.contains(&Column::KeySpec.id()),
            "KeySpec not default for AWS managed"
        );
        assert!(
            !ids.contains(&Column::ExpirationDate.id()),
            "ExpirationDate not default"
        );
    }

    #[test]
    fn test_customer_managed_defaults() {
        let ids = Column::customer_managed_default_visible();
        assert!(ids.contains(&Column::Alias.id()));
        assert!(ids.contains(&Column::KeyId.id()));
        assert!(ids.contains(&Column::KeyState.id()));
        assert!(ids.contains(&Column::KeySpec.id()));
        assert!(ids.contains(&Column::KeyUsage.id()));
        assert!(!ids.contains(&Column::ExpirationDate.id()));
        assert!(!ids.contains(&Column::Regionality.id()));
    }

    #[test]
    fn test_format_regionality() {
        assert_eq!(format_regionality(true), "Multi-Region");
        assert_eq!(format_regionality(false), "Single-Region");
    }

    #[test]
    fn test_format_key_type_symmetric() {
        assert_eq!(
            format_key_type("SymmetricDefault", "EncryptDecrypt"),
            "Symmetric"
        );
    }
}
