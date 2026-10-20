//! Presentation hints only: never authenticate, change permissions, or retry a read.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureKind {
    ExpiredCredentials,
    PermissionDenied,
    Other,
}

fn has_code(error: &str, code: &str) -> bool {
    // Match complete diagnostic tokens (AWS CLI parentheses / SDK quoted codes),
    // not substrings in resource names, ARNs, or URLs. Keep the original error
    // visible because text classification cannot replace typed provider errors.
    error.split_whitespace().any(|token| {
        token
            .trim_matches(['(', ')', '"', '\'', ':', ',', '[', ']', '.'])
            .eq_ignore_ascii_case(code)
    })
}

fn kind(error: &str) -> FailureKind {
    let lower = error.to_ascii_lowercase();
    if ["ExpiredToken", "ExpiredTokenException"]
        .iter()
        .any(|code| has_code(error, code))
        || lower.contains("the sso session associated with this profile has expired")
        || lower.contains("token has expired and refresh failed")
    {
        FailureKind::ExpiredCredentials
    } else if [
        "AccessDenied",
        "AccessDeniedException",
        "UnauthorizedOperation",
        "AuthorizationError",
        "AuthorizationErrorException",
        "ForbiddenException",
    ]
    .iter()
    .any(|code| has_code(error, code))
    {
        FailureKind::PermissionDenied
    } else {
        FailureKind::Other
    }
}

pub(crate) fn failure_text(error: Option<&str>) -> String {
    let clean: String = error
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();
    let clean = clean.trim();
    let detail = if clean.is_empty() {
        "The read did not complete successfully"
    } else {
        clean
    };
    let (heading, advice) = match kind(detail) {
        FailureKind::ExpiredCredentials => (
            "Read failed (credentials expired)",
            "Renew the credentials for this profile, then retry.\n",
        ),
        FailureKind::PermissionDenied => (
            "Read failed (permission denied)",
            "Check this profile's permissions for the requested read, then retry.\n",
        ),
        FailureKind::Other => ("Read failed", ""),
    };
    format!("{heading}: {detail}\n{advice}Ctrl+R: retry  Esc: back")
}

/// The caller describes the collection, while the shared renderer distinguishes
/// a successful empty collection from a local filter hiding loaded items.
pub(crate) struct EmptyState<'a> {
    pub no_items: &'a str,
    pub has_unfiltered_items: bool,
}

impl EmptyState<'_> {
    pub(super) fn text(&self) -> &str {
        if self.has_unfiltered_items {
            "No matching results. Clear or change the filter."
        } else {
            self.no_items
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_sdk_and_cli_error_codes_without_classifying_resource_substrings() {
        for code in ["ExpiredToken", "ExpiredTokenException"] {
            for error in [
                format!("{code}: The security token has expired"),
                format!("Failed read: service error: code: \"{code}\", message: expired"),
                format!("An error occurred ({code}) when calling ListTopics"),
            ] {
                assert_eq!(kind(&error), FailureKind::ExpiredCredentials, "{error}");
            }
        }
        for code in [
            "AccessDenied",
            "AccessDeniedException",
            "UnauthorizedOperation",
            "AuthorizationError",
            "AuthorizationErrorException",
            "ForbiddenException",
        ] {
            assert_eq!(
                kind(&format!("{code}: denied")),
                FailureKind::PermissionDenied
            );
        }
        for error in [
            "NoSuchEntity",
            "ResourceNotFoundException",
            "Unable to locate credentials",
            "InvalidClientTokenId",
            "SignatureDoesNotMatch",
            "RequestExpired",
            "HTTP 403",
            "read timed out after 30s",
            "missing AccessDenied-policy",
            "arn:aws:sns:us-east-1:123456789012:ExpiredToken",
            "https://example.invalid/AccessDenied",
            "service error",
        ] {
            assert_eq!(kind(error), FailureKind::Other, "{error}");
        }
    }

    #[test]
    fn sso_expiry_gets_manual_renewal_guidance_without_assuming_a_login_method() {
        for error in [
            "The SSO session associated with this profile has expired or is otherwise invalid",
            "Error when retrieving token from sso: Token has expired and refresh failed",
        ] {
            let text = failure_text(Some(error));
            assert!(text.contains("credentials expired") && text.contains("Renew"));
            assert!(text.contains(error) && text.contains("Ctrl+R: retry"));
            assert!(!text.contains("aws sso login"));
        }
    }

    #[test]
    fn failures_preserve_details_and_sanitize_control_characters() {
        let text = failure_text(Some("AccessDenied: denied\x1b[31m\nagain\r\t"));
        assert!(text.contains("permission denied") && text.contains("AccessDenied"));
        assert!(text.contains("permissions") && text.contains("Ctrl+R: retry"));
        assert!(!text.contains(['\x1b', '\r', '\t']));
        assert!(failure_text(Some("network down")).starts_with("Read failed: network down"));
        assert_eq!(failure_text(None), failure_text(Some(" \n\t")));
    }

    #[test]
    fn successful_empty_and_filtered_empty_have_different_messages() {
        let empty = EmptyState {
            no_items: "No topics found.",
            has_unfiltered_items: false,
        };
        assert_eq!(empty.text(), "No topics found.");
        let filtered = EmptyState {
            has_unfiltered_items: true,
            ..empty
        };
        assert_eq!(
            filtered.text(),
            "No matching results. Clear or change the filter."
        );
    }
}
