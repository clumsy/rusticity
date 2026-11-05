use crate::{
    common::{format_iso_timestamp, translate_column, ColumnId},
    ui::table::Column as TableColumn,
};
use ratatui::prelude::*;
use rusticity_core::Certificate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    CertificateId,
    DomainName,
    Type,
    Status,
    InUse,
    RenewalEligibility,
    KeyAlgorithm,
    AdditionalNames,
    NotBefore,
    NotAfter,
    RequestedAt,
    IssuedAt,
    ImportedAt,
    RevokedAt,
    Exported,
    ManagedBy,
}

impl Column {
    pub const ALL: [Self; 16] = [
        Self::CertificateId,
        Self::DomainName,
        Self::Type,
        Self::Status,
        Self::InUse,
        Self::RenewalEligibility,
        Self::KeyAlgorithm,
        Self::AdditionalNames,
        Self::NotBefore,
        Self::NotAfter,
        Self::RequestedAt,
        Self::IssuedAt,
        Self::ImportedAt,
        Self::RevokedAt,
        Self::Exported,
        Self::ManagedBy,
    ];

    fn definition(self) -> (ColumnId, &'static str, u16) {
        let ts = crate::common::UTC_TIMESTAMP_WIDTH;
        match self {
            Self::CertificateId => ("column.acm.certificate.id", "Certificate ID", 38),
            Self::DomainName => ("column.acm.certificate.domain", "Domain name", 40),
            Self::Type => ("column.acm.certificate.type", "Type", 16),
            Self::Status => ("column.acm.certificate.status", "Status", 22),
            Self::InUse => ("column.acm.certificate.in_use", "In use", 8),
            Self::RenewalEligibility => (
                "column.acm.certificate.renewal_eligibility",
                "Renewal eligibility",
                20,
            ),
            Self::KeyAlgorithm => ("column.acm.certificate.key_algorithm", "Key algorithm", 16),
            Self::AdditionalNames => (
                "column.acm.certificate.additional_names",
                "Additional names",
                40,
            ),
            Self::NotBefore => ("column.acm.certificate.not_before", "Not before", ts),
            Self::NotAfter => ("column.acm.certificate.not_after", "Not after", ts),
            Self::RequestedAt => ("column.acm.certificate.requested_at", "Requested at", ts),
            Self::IssuedAt => ("column.acm.certificate.issued_at", "Issued at", ts),
            Self::ImportedAt => ("column.acm.certificate.imported_at", "Imported at", ts),
            Self::RevokedAt => ("column.acm.certificate.revoked_at", "Revoked at", ts),
            Self::Exported => ("column.acm.certificate.exported", "Exported", 10),
            Self::ManagedBy => ("column.acm.certificate.managed_by", "Managed by", 16),
        }
    }

    pub fn id(self) -> ColumnId {
        self.definition().0
    }
    pub fn default_name(self) -> &'static str {
        self.definition().1
    }
    pub fn name(self) -> String {
        translate_column(self.id(), self.default_name())
    }
    pub fn ids() -> Vec<ColumnId> {
        Self::ALL.into_iter().map(Self::id).collect()
    }
    pub fn default_visible_ids() -> Vec<ColumnId> {
        Self::ALL[..7].iter().map(|c| c.id()).collect()
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }

    /// The text this column shows, also used by the inventory filter so that
    /// what you search for is what you see.
    pub fn text(self, cert: &Certificate) -> String {
        match self {
            Self::CertificateId => cert.id.clone(),
            Self::DomainName => cert.domain_name.clone(),
            Self::Type => label(&cert.cert_type),
            Self::Status => label(&cert.status),
            // The console shows No for a certificate that is not in use.
            Self::InUse => yes_no(cert.in_use).into(),
            Self::RenewalEligibility => label(&cert.renewal_eligibility),
            Self::KeyAlgorithm => cert.key_algorithm.replace('_', " "),
            Self::AdditionalNames => additional_names(cert),
            Self::NotBefore => timestamp(&cert.not_before),
            Self::NotAfter => timestamp(&cert.not_after),
            Self::RequestedAt => timestamp(&cert.created_at),
            Self::IssuedAt => timestamp(&cert.issued_at),
            Self::ImportedAt => timestamp(&cert.imported_at),
            Self::RevokedAt => timestamp(&cert.revoked_at),
            Self::Exported => yes_no(cert.exported).into(),
            Self::ManagedBy => label(&cert.managed_by),
        }
    }

    pub fn render(self, cert: &Certificate) -> (String, Style) {
        let text = self.text(cert);
        let text: String = if text.is_empty() {
            "-".into()
        } else {
            text.chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect()
        };
        let style = match self {
            Self::Status => Style::default().fg(status_color(&cert.status)),
            _ => Style::default(),
        };
        (text, style)
    }

    pub fn into_table_column(self) -> Box<dyn TableColumn<Certificate>> {
        Box::new(crate::ui::table::OwnedColumn::new(
            self.id(),
            self.default_name(),
            self.definition().2,
            !matches!(self, Self::DomainName | Self::AdditionalNames),
            move |cert: &Certificate| self.render(cert),
        ))
    }
}

/// `PENDING_VALIDATION` → `Pending validation`, `AMAZON_ISSUED` → `Amazon issued`.
fn label(wire: &str) -> String {
    let lower = wire.replace('_', " ").to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn yes_no(flag: Option<bool>) -> &'static str {
    if flag == Some(true) {
        "Yes"
    } else {
        "No"
    }
}

fn timestamp(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format_iso_timestamp(value)
    }
}

/// Names other than the primary domain; `…` when AWS truncated the list.
fn additional_names(cert: &Certificate) -> String {
    let mut names: Vec<&str> = cert
        .additional_names
        .iter()
        .map(String::as_str)
        .filter(|name| *name != cert.domain_name)
        .collect();
    if cert.more_names {
        names.push("…");
    }
    names.join(", ")
}

fn status_color(status: &str) -> Color {
    match status {
        "ISSUED" => Color::Green,
        "PENDING_VALIDATION" | "INACTIVE" => Color::Yellow,
        "EXPIRED" | "REVOKED" | "FAILED" | "VALIDATION_TIMED_OUT" => Color::Red,
        _ => Color::White,
    }
}
