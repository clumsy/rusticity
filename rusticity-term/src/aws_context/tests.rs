use super::*;

#[test]
fn an_offline_context_holds_the_profile_and_region_and_no_identity() {
    let aws = AwsContext::offline("beta".into(), Some("us-west-2".into()));
    assert_eq!(aws.profile, "beta");
    assert_eq!(aws.config.region, "us-west-2");
    assert!(aws.config.account_id.is_empty() && aws.config.role_arn.is_empty());
}

#[test]
fn a_reconnect_request_names_the_same_profile_and_region() {
    let aws = AwsContext::offline("beta".into(), Some("us-west-2".into()));
    assert_eq!(
        aws.request(),
        ContextRequest {
            profile: Some("beta".into()),
            region: Some("us-west-2".into()),
        }
    );
    // Unset parts stay unset, so the environment and profile default apply.
    let aws = AwsContext::offline(String::new(), None);
    assert_eq!(
        aws.request(),
        ContextRequest {
            profile: None,
            region: None,
        }
    );
}
