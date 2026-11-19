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

#[test]
fn another_region_keeps_the_sign_in_and_builds_its_clients_there() {
    let mut aws = AwsContext::offline("beta".into(), Some("us-east-1".into()));
    aws.connected = true;
    aws.config.account_id = "123456789012".into();
    let moved = aws.in_region("eu-west-1");
    assert_eq!(moved.profile, "beta");
    assert!(moved.connected);
    assert_eq!(moved.config.region, "eu-west-1");
    assert_eq!(moved.config.account_id, "123456789012");
    assert_eq!(moved.cloudwatch_client.config().region, "eu-west-1");
    assert_eq!(aws.config.region, "us-east-1", "the original is untouched");
    let copy = aws.duplicate();
    assert_eq!(copy.config.region, "us-east-1");
    assert!(!AwsContext::offline("beta".into(), None).connected);
}
