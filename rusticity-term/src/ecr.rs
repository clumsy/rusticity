pub mod image;
pub mod repo;

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
