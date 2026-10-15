//! Replay the same boundary cases through all three streamed AWS readers.
use super::*;
use serde_json::json;

#[derive(Clone, Copy, Debug)]
enum Reader {
    Lambda,
    QueueConsumers,
    Stack,
}
const READERS: [Reader; 3] = [Reader::Lambda, Reader::QueueConsumers, Reader::Stack];

/// Each tuple is (source row count, returned continuation token). Negative count
/// injects a later-page API failure; emit=false exercises unresolvable source rows.
async fn replay(
    reader: Reader,
    pages: Vec<(i32, Option<&str>)>,
    emit: bool,
    deny_attributes: bool,
) -> (Vec<Resource>, Vec<String>, bool, Vec<String>) {
    let pages: Vec<_> = pages
        .into_iter()
        .map(|(n, t)| (n, t.map(str::to_owned)))
        .collect();
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap_or_default()).unwrap();
        let target = request
            .headers()
            .get("x-amz-target")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        let json_response = |status, value: serde_json::Value| {
            http::Response::builder()
                .status(status)
                .header("content-type", "application/x-amz-json-1.0")
                .body(value.to_string().into_bytes())
                .unwrap()
        };
        if target == "AmazonSQS.GetQueueUrl" {
            return json_response(
                200,
                json!({"QueueUrl":"https://sqs.us-east-1.amazonaws.com/123456789012/source"}),
            );
        }
        if target == "AmazonSQS.GetQueueAttributes" {
            return if deny_attributes {
                json_response(
                    400,
                    json!({"__type":"AccessDeniedException","message":"denied"}),
                )
            } else {
                json_response(
                    200,
                    json!({"Attributes":{"QueueArn":"arn:aws:sqs:us-east-1:123456789012:source"}}),
                )
            };
        }
        if request.uri().path().contains("/functions/") {
            return json_response(200, json!({"FunctionName":"source"}));
        }
        match reader {
            Reader::Stack => assert!(body.contains("Action=ListStackResources")),
            _ => assert!(request.uri().path().contains("event-source-mappings")),
        }
        let mut requests = captured.lock().unwrap();
        let index = requests.len();
        requests.push(format!("{} {body}", request.uri()));
        let (count, token) = pages.get(index).expect("unexpected extra page request");
        if *count < 0 {
            return if matches!(reader, Reader::Stack) {
                http::Response::builder().status(403).body(b"<ErrorResponse><Error><Code>AccessDenied</Code><Message>denied</Message></Error></ErrorResponse>".to_vec()).unwrap()
            } else {
                json_response(403, json!({"Type":"User","message":"denied"}))
            };
        }
        if matches!(reader, Reader::Stack) {
            let rows: String = (0..*count).map(|i| if emit {
                format!("<member><PhysicalResourceId>bucket-{index}-{i}</PhysicalResourceId><ResourceType>AWS::S3::Bucket</ResourceType></member>")
            } else { "<member><ResourceType>AWS::Custom::Resource</ResourceType></member>".into() }).collect();
            let token = token
                .as_ref()
                .map(|token| format!("<NextToken>{token}</NextToken>"))
                .unwrap_or_default();
            return http::Response::builder().status(200).body(format!("<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries>{rows}</StackResourceSummaries>{token}</ListStackResourcesResult></ListStackResourcesResponse>").into_bytes()).unwrap();
        }
        let rows: Vec<_> = (0..*count).map(|i| {
            if !emit { return json!({}); }
            match reader {
                Reader::Lambda => json!({"EventSourceArn":format!("arn:aws:sqs:us-east-1:123456789012:queue-{index}-{i}")}),
                _ => json!({"FunctionArn":format!("arn:aws:lambda:us-east-1:123456789012:function:fn-{index}-{i}")}),
            }
        }).collect();
        json_response(200, json!({"EventSourceMappings":rows,"NextMarker":token}))
    });
    let arn = match reader {
        Reader::Lambda => "arn:aws:lambda:us-east-1:123456789012:function:source",
        Reader::QueueConsumers => "arn:aws:sqs:us-east-1:123456789012:source",
        Reader::Stack => "arn:aws:cloudformation:us-east-1:123456789012:stack/source/uuid",
    };
    let (tx, mut rx) = mpsc::channel(1);
    let task = tokio::spawn(discover(config, Resource::from_arn(arn).unwrap(), tx));
    let mut resources = vec![];
    let mut warnings = vec![];
    let mut finished = None;
    while let Some(update) = rx.recv().await {
        match update {
            SearchUpdate::Results(rows) => resources.extend(rows),
            SearchUpdate::Warning(warning) => warnings.push(warning),
            SearchUpdate::Finished { limited } => finished = Some(limited),
            _ => {}
        }
    }
    task.await.unwrap();
    let requests = requests.lock().unwrap().clone();
    (
        resources,
        warnings,
        finished.expect("must deliver Finished"),
        requests,
    )
}

#[tokio::test]
async fn all_streamed_readers_keep_page_rows_before_cycle_warning() {
    for reader in READERS {
        let (rows, warnings, limited, requests) = replay(
            reader,
            vec![(1, Some("a")), (1, Some("b")), (1, Some("a"))],
            true,
            false,
        )
        .await;
        assert_eq!(rows.len(), 3, "{reader:?}");
        assert_eq!(
            warnings,
            ["Repeated pagination token; related results are incomplete"]
        );
        assert!(limited);
        assert_eq!(requests.len(), 3);
        let param = if matches!(reader, Reader::Stack) {
            "NextToken"
        } else {
            "Marker"
        };
        assert!(requests[1].contains(&format!("{param}=a")));
        assert!(requests[2].contains(&format!("{param}=b")));
    }
}

#[tokio::test]
async fn all_streamed_readers_preserve_final_page_and_over_limit_page_semantics() {
    for reader in READERS {
        for more in [false, true] {
            let (rows, warnings, limited, requests) = replay(
                reader,
                vec![(600, Some("a")), (600, more.then_some("b"))],
                true,
                false,
            )
            .await;
            assert_eq!(
                rows.len(),
                1200,
                "streamed page must not be silently truncated: {reader:?}"
            );
            assert_eq!(requests.len(), 2);
            assert_eq!(limited, more);
            if more {
                assert_eq!(
                    warnings,
                    ["Pagination capped at 1000 results (may be incomplete)"]
                );
            } else {
                assert!(warnings.is_empty());
            }
        }
    }
}

#[tokio::test]
async fn all_streamed_readers_count_source_rows_not_only_emitted_references() {
    for reader in READERS {
        let (rows, warnings, limited, requests) =
            replay(reader, vec![(1000, Some("a"))], false, false).await;
        assert!(rows.is_empty());
        assert_eq!(requests.len(), 1);
        assert!(limited);
        assert_eq!(
            warnings,
            ["Pagination capped at 1000 results (may be incomplete)"]
        );
    }
}

#[tokio::test]
async fn all_streamed_readers_keep_rows_and_finish_when_later_page_fails() {
    for reader in READERS {
        let (rows, warnings, limited, requests) =
            replay(reader, vec![(2, Some("a")), (-1, None)], true, false).await;
        assert_eq!(rows.len(), 2);
        assert!(limited);
        assert_eq!(requests.len(), 2);
        assert!(warnings
            .iter()
            .any(|w| w.starts_with("Related resource lookup:")));
    }
}

#[tokio::test]
async fn queue_consumers_preserve_prior_attribute_failure_on_successful_final_page() {
    let (rows, warnings, limited, requests) =
        replay(Reader::QueueConsumers, vec![(1, None)], true, true).await;
    assert_eq!(rows.len(), 1);
    assert!(limited);
    assert_eq!(requests.len(), 1);
    assert!(
        warnings.iter().any(|w| w.contains("SQS attributes:")),
        "{warnings:?}"
    );
}
