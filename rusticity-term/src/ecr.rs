pub mod actions;
pub mod image;
pub mod repo;
pub mod scan;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    repo::init(i18n);
    image::init(i18n);
}

pub fn console_url_repositories(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/ecr/private-registry/repositories?region={}",
        region, region
    )
}

pub fn console_url_private_repository(region: &str, account_id: &str, repo_name: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/ecr/repositories/private/{}/{}?region={}",
        region, account_id, repo_name, region
    )
}

pub fn console_url_public_repository(region: &str, repo_name: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/ecr/repositories/public/{}?region={}",
        region, repo_name, region
    )
}

/// Canonical regional console link. Account-specific sign-in host prefixes are
/// browser/session context, not part of the AWS resource identity.
pub fn console_url_private_image(
    region: &str,
    account_id: &str,
    repo_name: &str,
    digest: &str,
) -> String {
    format!(
        "https://{region}.console.aws.amazon.com/ecr/repositories/private/{account_id}/{}/_/image/{}/details?region={region}",
        urlencoding::encode(repo_name),
        urlencoding::encode(digest),
    )
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
}
