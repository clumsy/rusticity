use super::*;
use crate::finder::{Resource, ResourceKind};
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::Mutex as StdMutex;

fn query(input: &str) -> SearchQuery {
    SearchQuery::parse(input).unwrap()
}

fn row(kind: ResourceKind, id: &str, region: &str) -> Resource {
    Resource {
        kind,
        id: id.into(),
        name: id.into(),
        arn: String::new(),
        region: region.into(),
    }
}

async fn collect(mut rx: mpsc::Receiver<SearchUpdate>) -> Vec<SearchUpdate> {
    let mut out = Vec::new();
    while let Some(update) = rx.recv().await {
        out.push(update);
    }
    out
}

#[test]
fn region_qualifier_parses_lists_all_and_rejects_bad_codes() {
    assert_eq!(query("orders").regions, RegionScope::Current);
    assert_eq!(
        query("orders region:eu-west-1,US-EAST-2,eu-west-1").regions,
        RegionScope::List(vec!["eu-west-1".into(), "us-east-2".into()]),
        "lowercased, de-duplicated, in typed order"
    );
    assert_eq!(query("Region:all orders").regions, RegionScope::All);
    assert_eq!(query("orders region:*").regions, RegionScope::All);
    assert_eq!(
        query("region:us-gov-west-1,cn-northwest-1 orders").regions,
        RegionScope::List(vec!["us-gov-west-1".into(), "cn-northwest-1".into()])
    );
    let q = query("@sns orders region:us-east-1");
    assert_eq!(q.text, "orders", "the qualifier is not search text");
    assert_eq!(q.kinds, vec![ResourceKind::Topic]);
    for bad in [
        "region:",
        "region:useast1",
        "region:us-east",
        "region:us-east-x",
        "region:eu-west-1;rm",
    ] {
        assert!(
            SearchQuery::parse(&format!("orders {bad}")).is_err(),
            "{bad}"
        );
    }
    let many = (1..=41)
        .map(|i| format!("us-east-{i}"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(SearchQuery::parse(&format!("orders region:{many}")).is_err());
}

#[test]
fn index_query_asks_for_global_resources_only_when_told_to() {
    let mut q = query("orders");
    assert!(q
        .index_query("us-east-1")
        .ends_with("region:us-east-1,global"));
    q.include_global = false;
    assert!(q.index_query("eu-west-1").ends_with("region:eu-west-1"));
}

#[tokio::test]
async fn fan_out_merges_regions_prefixes_warnings_and_finishes_once() {
    type Calls = Vec<(String, Vec<ResourceKind>, bool)>;
    let seen: Arc<StdMutex<Calls>> = Arc::default();
    let (tx, rx) = mpsc::channel(64);
    let record = seen.clone();
    fan_out(
        vec!["us-east-1".into(), "eu-west-1".into(), "ap-south-1".into()],
        query("orders"),
        tx,
        move |region, query, tx| {
            record.lock().unwrap().push((
                region.clone(),
                query.kinds.clone(),
                query.include_global,
            ));
            async move {
                let _ = tx.send(SearchUpdate::Provider("ignored")).await;
                if region == "eu-west-1" {
                    let _ = tx.send(SearchUpdate::Warning("AccessDenied".into())).await;
                    let _ = tx.send(SearchUpdate::Finished { limited: false }).await;
                    return;
                }
                let _ = tx
                    .send(SearchUpdate::Results(vec![row(
                        ResourceKind::Topic,
                        "orders",
                        &region,
                    )]))
                    .await;
                let _ = tx
                    .send(SearchUpdate::Finished {
                        limited: region == "ap-south-1",
                    })
                    .await;
            }
        },
    )
    .await;
    let updates = collect(rx).await;

    let results: Vec<_> = updates
        .iter()
        .filter_map(|u| match u {
            SearchUpdate::Results(rows) => Some(rows[0].region.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(results.len(), 2, "one failing Region hides nothing else");
    assert!(results.contains(&"us-east-1".into()) && results.contains(&"ap-south-1".into()));
    assert!(updates
        .iter()
        .any(|u| matches!(u, SearchUpdate::Warning(w) if w == "eu-west-1: AccessDenied")));
    let finished: Vec<_> = updates
        .iter()
        .filter_map(|u| match u {
            SearchUpdate::Finished { limited } => Some(*limited),
            _ => None,
        })
        .collect();
    assert_eq!(
        finished,
        [true],
        "a single Finished, limited if any Region was"
    );
    assert!(matches!(
        updates[0],
        SearchUpdate::Provider("Parallel multi-Region search")
    ));
    assert!(!updates
        .iter()
        .any(|u| matches!(u, SearchUpdate::Provider("ignored"))));

    // Global inventories are searched in the first Region only.
    let seen = seen.lock().unwrap();
    let first = seen.iter().find(|(r, ..)| r == "us-east-1").unwrap();
    assert!(first.1.contains(&ResourceKind::User) && first.2);
    for (region, kinds, global) in seen.iter().filter(|(r, ..)| r != "us-east-1") {
        assert!(!global, "{region}");
        assert!(!kinds.iter().any(|k| k.is_global()), "{region}: {kinds:?}");
        assert!(kinds.contains(&ResourceKind::Topic));
    }
}

#[tokio::test]
async fn global_only_queries_run_in_the_first_region_alone() {
    let regions: Arc<StdMutex<Vec<String>>> = Arc::default();
    let (tx, rx) = mpsc::channel(64);
    let record = regions.clone();
    fan_out(
        vec!["us-east-1".into(), "eu-west-1".into()],
        query("@iam admin"),
        tx,
        move |region, _query, tx| {
            record.lock().unwrap().push(region);
            async move {
                let _ = tx.send(SearchUpdate::Finished { limited: false }).await;
            }
        },
    )
    .await;
    assert_eq!(*regions.lock().unwrap(), ["us-east-1"]);
    assert_eq!(collect(rx).await.len(), 2, "provider + finished");
}

#[tokio::test]
async fn a_closed_finder_stops_every_region_without_hanging() {
    let (tx, rx) = mpsc::channel(1);
    drop(rx);
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        fan_out(
            vec!["us-east-1".into(), "eu-west-1".into()],
            query("orders"),
            tx,
            |region, _query, tx| async move {
                // A chatty Region: would block forever if nobody drained it.
                for i in 0..1000 {
                    if tx
                        .send(SearchUpdate::Results(vec![row(
                            ResourceKind::Topic,
                            &format!("t{i}"),
                            &region,
                        )]))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            },
        ),
    )
    .await;
    assert!(result.is_ok(), "fan_out returned");
}

#[tokio::test]
async fn a_cancelled_search_mid_stream_stops_without_hanging() {
    let (tx, mut rx) = mpsc::channel(1);
    let search = fan_out(
        vec!["us-east-1".into(), "eu-west-1".into()],
        query("orders"),
        tx,
        |region, _query, tx| async move {
            for i in 0..1000 {
                if tx
                    .send(SearchUpdate::Results(vec![row(
                        ResourceKind::Topic,
                        &format!("t{i}"),
                        &region,
                    )]))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        },
    );
    let reader = async {
        // Take a few updates, then go away like a closed finder.
        for _ in 0..3 {
            rx.recv().await;
        }
        drop(rx);
    };
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(search, reader);
    })
    .await;
    assert!(result.is_ok());
}

fn ec2_config(body: &'static str, status: u16) -> AwsConfig {
    let http = infallible_client_fn(move |_request| {
        http::Response::builder()
            .status(status)
            .body(body.as_bytes().to_vec())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("eu-west-1"))
        .credentials_provider(aws_sdk_ec2::config::SharedCredentialsProvider::new(
            aws_sdk_ec2::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    AwsConfig::from_sdk_config(sdk_config, "eu-west-1", "123456789012")
}

#[tokio::test]
async fn region_all_uses_enabled_regions_with_the_active_region_first() {
    let config = ec2_config(
        r#"<DescribeRegionsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
  <requestId>r</requestId>
  <regionInfo>
    <item><regionName>us-east-1</regionName></item>
    <item><regionName>eu-west-1</regionName></item>
    <item><regionName>ap-south-1</regionName></item>
    <item><regionName>us-east-1</regionName></item>
  </regionInfo>
</DescribeRegionsResponse>"#,
        200,
    );
    assert_eq!(
        enabled_regions(&config).await.unwrap(),
        ["eu-west-1", "ap-south-1", "us-east-1"]
    );
}

#[tokio::test]
async fn region_all_falls_back_to_the_active_region_with_a_warning() {
    let config = ec2_config(
        r#"<Response><Errors><Error><Code>UnauthorizedOperation</Code><Message>denied</Message></Error></Errors><RequestID>r</RequestID></Response>"#,
        403,
    );
    assert!(enabled_regions(&config).await.is_err());
    let (tx, mut rx) = mpsc::channel(64);
    // Stop right after the fallback warning; the single-Region search that
    // follows would need real inventories.
    let task = tokio::spawn(search_regions(
        config,
        query("orders region:all"),
        SharedRegionCaches::default(),
        tx,
    ));
    let first = rx.recv().await.unwrap();
    task.abort();
    assert!(
        matches!(&first, SearchUpdate::Warning(w) if w.contains("searching eu-west-1 only")),
        "{first:?}"
    );
}

#[test]
fn for_region_keeps_identity_and_changes_only_the_region() {
    let config = ec2_config("", 200);
    let other = config.for_region("ap-south-1");
    assert_eq!(other.region, "ap-south-1");
    assert_eq!(other.account_id, config.account_id);
    assert_eq!(
        other.ec2_client().config().region().map(|r| r.as_ref()),
        Some("ap-south-1")
    );
}
