//! Opt-in executable AWS CLI contracts. Every invocation targets an explicit
//! loopback mock with dummy credentials. No AWS service is contacted.
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn targets() -> Vec<Target> {
    let mut targets = Vec::new();
    for kind in ResourceKind::ALL {
        if kind == ResourceKind::Api {
            continue;
        }
        let id = match kind {
            ResourceKind::Instance => "i-0123456789abcdef0",
            ResourceKind::Vpc => "vpc-0123456789abcdef0",
            ResourceKind::Efs | ResourceKind::Fsx => "fs-0123456789abcdef0",
            ResourceKind::Key => "12345678-1234-1234-1234-123456789012",
            ResourceKind::LogGroup => "/aws/lambda/test",
            _ => "selected-resource",
        };
        let mut resource = resource(kind);
        resource.id = id.into();
        if kind == ResourceKind::Topic {
            resource.arn = "arn:aws:sns:us-east-1:123456789012:selected-resource".into();
        }
        targets.push(Target::Resource(resource));
    }
    targets.push(Target::EcrImage {
        repository: "team/image".into(),
        digest: format!("sha256:{}", "a".repeat(64)),
    });
    targets.push(Target::S3Object {
        bucket: "test-bucket".into(),
        key: "test-object".into(),
        region: "us-east-1".into(),
    });
    for protocol in ["REST", "HTTP", "WEBSOCKET"] {
        targets.push(Target::Api {
            id: "abc123".into(),
            protocol: protocol.into(),
        });
    }
    targets
}

#[test]
fn every_command_uses_separate_global_json_argument_and_valid_json() {
    for target in targets() {
        let invocation = invocation(&target, context()).unwrap();
        let command = command(&target, context()).unwrap();
        assert_eq!(
            invocation.args,
            words(&command)[1..],
            "copy and execution must use identical arguments"
        );
        assert!(!command.contains("--cli-input-json="));
        assert!(input(&command).is_object());
    }
}

fn capture_request(command: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let argv = words(command);
    let mut child = Command::new("aws")
        .args(&argv[1..])
        .args([
            "--endpoint-url",
            &format!("http://{address}"),
            "--cli-connect-timeout",
            "2",
            "--cli-read-timeout",
            "2",
        ])
        .env("AWS_EC2_METADATA_DISABLED", "true")
        .env("AWS_CONFIG_FILE", "/dev/null")
        .env("AWS_SHARED_CREDENTIALS_FILE", "/dev/null")
        .env("AWS_ACCESS_KEY_ID", "test")
        .env("AWS_SECRET_ACCESS_KEY", "test")
        .env("AWS_DEFAULT_REGION", "us-east-1")
        .env("AWS_PAGER", "")
        .env("AWS_CLI_AUTO_PROMPT", "off")
        .env("AWS_MAX_ATTEMPTS", "1")
        .env("NO_PROXY", "127.0.0.1")
        .env_remove("AWS_PROFILE")
        .env_remove("AWS_SESSION_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("AWS CLI is required for this explicit contract test");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("Mock listener: {error}"),
        }
        if child.try_wait().unwrap().is_some() {
            let output = child.wait_with_output().unwrap();
            panic!(
                "CLI rejected command before request: {command}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI did not reach mock endpoint: {command}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut request = Vec::new();
    loop {
        let mut buf = [0; 4096];
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "request closed early");
        request.extend_from_slice(&buf[..n]);
        if let Some(header_end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&request[..header_end]);
            let length = header
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + length {
                break;
            }
        }
    }
    // A deliberate service error ends every protocol after request capture;
    // the test is about parser/serialization contracts, not mock business data.
    let body = "{\"__type\":\"ContractCheck\",\"message\":\"local fixture\"}";
    let reply = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/x-amz-json-1.1\r\nx-amzn-ErrorType: ContractCheck\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
    stream.write_all(reply.as_bytes()).unwrap();
    drop(stream);
    let finish_deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > finish_deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI did not finish after fixture response");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = child.wait_with_output().unwrap();
    let request = String::from_utf8(request).unwrap();
    assert!(request.to_lowercase().contains(&format!("host: {address}")));
    request
}

#[test]
#[ignore = "requires AWS CLI on PATH; localhost mock only, dummy credentials"]
fn installed_aws_cli_accepts_every_generated_command() {
    let version = Command::new("aws").arg("--version").output().unwrap();
    assert!(version.status.success());
    eprintln!("{}", String::from_utf8_lossy(&version.stdout).trim());
    for target in targets() {
        let command = command(
            &target,
            Context {
                profile: "",
                ..context()
            },
        )
        .unwrap();
        let request = capture_request(&command);
        let (_, body) = request.split_once("\r\n\r\n").unwrap();
        let expected = input(&command);
        // JSON protocol commands preserve the complete selected-resource input.
        if body.starts_with('{') {
            assert_eq!(serde_json::from_str::<Value>(body).unwrap(), expected);
        } else {
            // Query/REST protocols encode identifiers in form parameters or URI.
            let decoded = urlencoding::decode(&request).unwrap();
            fn strings(value: &Value, result: &mut Vec<String>) {
                match value {
                    Value::String(s) => result.push(s.clone()),
                    Value::Array(a) => {
                        for v in a {
                            strings(v, result);
                        }
                    }
                    Value::Object(o) => {
                        for v in o.values() {
                            strings(v, result);
                        }
                    }
                    _ => {}
                }
            }
            let mut values = vec![];
            strings(&expected, &mut values);
            for value in values {
                assert!(
                    decoded.contains(&value),
                    "Missing literal {value:?} in request for {command}"
                );
            }
        }
        let argv = words(&command);
        eprintln!("PASS {} {}", argv[1], argv[2]);
    }
    for key in [
        "it's a file",
        "file:///does-not-exist",
        "fileb:///does-not-exist",
        "--profile=evil",
        "$(touch bad);echo test",
        "a,b=c",
        "[one,two]",
        "a\\b\"c",
        "資源",
    ] {
        let command = command(
            &Target::S3Object {
                bucket: "test-bucket".into(),
                key: key.into(),
                region: "us-east-1".into(),
            },
            Context {
                profile: "",
                ..context()
            },
        )
        .unwrap();
        let request = capture_request(&command);
        let path = request
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        assert_eq!(
            urlencoding::decode(path).unwrap(),
            format!("/test-bucket/{key}")
        );
    }
}
