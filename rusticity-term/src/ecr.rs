pub mod actions;
pub mod image;
pub mod repo;
pub mod scan;
use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    repo::init(i18n);
    image::init(i18n);
}

pub fn console_url_repositories(region: &str) -> String {
    ConsoleUrl::regional(region, "/ecr/private-registry/repositories")
        .query("region", region)
        .finish()
}

pub fn console_url_private_repository(region: &str, account_id: &str, repo_name: &str) -> String {
    ConsoleUrl::regional(
        region,
        &format!(
            "/ecr/repositories/private/{}/{}",
            component(account_id),
            component(repo_name)
        ),
    )
    .query("region", region)
    .finish()
}

pub fn console_url_public_repository(region: &str, repo_name: &str) -> String {
    ConsoleUrl::regional(
        region,
        &format!("/ecr/repositories/public/{}", component(repo_name)),
    )
    .query("region", region)
    .finish()
}

/// Canonical regional console link. Account-specific sign-in host prefixes are
/// browser/session context, not part of the AWS resource identity.
pub fn console_url_private_image(
    region: &str,
    account_id: &str,
    repo_name: &str,
    digest: &str,
) -> String {
    ConsoleUrl::regional(
        region,
        &format!(
            "/ecr/repositories/private/{}/{}/_/image/{}/details",
            component(account_id),
            component(repo_name),
            component(digest),
        ),
    )
    .query("region", region)
    .finish()
}

/// Public repository inventory entry point documented by ECR Public. Unlike the
/// private-registry route, this does not force the private registry section.
pub fn console_url_public_repositories(region: &str) -> String {
    ConsoleUrl::regional(region, "/ecr/repositories")
        .query("region", region)
        .finish()
}

#[cfg(test)]
mod console_tests {
    use super::*;

    #[test]
    fn image_console_url_uses_current_context_and_encodes_path_segments() {
        assert_eq!(
            console_url_private_image("eu-west-1", "123456789012", "team/image", "sha256:abc123"),
            "https://eu-west-1.console.aws.amazon.com/ecr/repositories/private/123456789012/team%2Fimage/_/image/sha256%3Aabc123/details?region=eu-west-1"
        );
        let url = console_url_private_image("us-west-2", "999999999999", "repo?x#y", "sha256:a/b");
        assert!(url.contains("/999999999999/repo%3Fx%23y/_/image/sha256%3Aa%2Fb/details"));
        assert!(url.ends_with("?region=us-west-2"));
    }

    #[test]
    fn repo_slash_encoded_once_no_double_encode_of_existing_digest() {
        // Slash in repo name is encoded once; an already-escaped-looking digest
        // (literal '%' in sha256:...) is treated as raw and escaped exactly once,
        // never double-encoded.
        let url = console_url_private_image(
            "us-east-1",
            "111111111111",
            "team/sub/repo",
            "sha256:dead%beef",
        );
        assert!(url.contains("/team%2Fsub%2Frepo/_/image/sha256%3Adead%25beef/details"));
        // No double-encoding: the single-encoded forms do not reappear encoded again.
        assert!(!url.contains("%252F"));
        assert!(!url.contains("%253A"));
    }
}
