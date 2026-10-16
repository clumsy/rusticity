//! Bounded subprocess runner. Only trusted cli_command argv reaches `run`.
use std::{io, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

const OUTPUT_LIMIT: usize = 1024 * 1024;
const ERROR_LIMIT: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

pub(super) async fn run(args: Vec<String>) -> Result<String, String> {
    let mut command = Command::new("aws");
    command
        .args(args)
        .args([
            "--output",
            "json",
            "--no-cli-pager",
            "--no-cli-auto-prompt",
            "--cli-connect-timeout",
            "5",
            "--cli-read-timeout",
            "20",
        ])
        .env("AWS_PAGER", "")
        .env("PAGER", "cat")
        .env("AWS_CLI_AUTO_PROMPT", "off");
    execute(command, TIMEOUT).await
}

async fn bounded_read(mut reader: impl AsyncRead + Unpin, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
            return Ok(bytes);
        }
        if bytes.len() + n > limit {
            return Err(io::Error::other(format!("CLI output exceeded its {limit}-byte limit; copy the command with Ctrl+Y and run it in your terminal")));
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
}

async fn execute(mut command: Command, timeout: Duration) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => "AWS CLI is not installed or not on PATH".into(),
            _ => format!("Could not start CLI: {error}"),
        })?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let operation = async {
        tokio::try_join!(
            bounded_read(stdout, OUTPUT_LIMIT),
            bounded_read(stderr, ERROR_LIMIT),
            child.wait()
        )
    };
    let result = tokio::time::timeout(timeout, operation).await;
    let (stdout, stderr, status) = match result {
        Ok(Ok(result)) => result,
        error => {
            // Reap explicitly on bounds/timeouts. Dropping this future on Esc
            // also terminates the immediate CLI child via kill_on_drop.
            let _ = child.kill().await;
            return Err(match error {
                Err(_) => format!("CLI request timed out after {} seconds", timeout.as_secs()),
                Ok(Err(error)) => error.to_string(),
                _ => unreachable!(),
            });
        }
    };
    if !status.success() {
        let error = clean(&String::from_utf8_lossy(&stderr));
        return Err(if error.trim().is_empty() {
            format!("CLI exited with {status}")
        } else {
            error
        });
    }
    let value: serde_json::Value = serde_json::from_slice(&stdout)
        .map_err(|_| "CLI succeeded but did not return valid JSON".to_string())?;
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    if text.len() > OUTPUT_LIMIT * 2 {
        return Err("Formatted JSON exceeds the popup limit; copy the command and inspect it in your terminal".into());
    }
    Ok(text)
}

pub(super) fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn python(script: &str) -> Command {
        let mut command = Command::new("python3");
        command.args(["-c", script]);
        command
    }
    #[tokio::test]
    async fn pretty_prints_json_and_escapes_control_characters() {
        let text = execute(
            python("print('{\"a\": [1, 2], \"escape\": \"\\\\u001b\"}')"),
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert!(text.contains("\n  \"a\": ["));
        assert!(!text.contains('\x1b'));
    }
    #[tokio::test]
    async fn service_errors_and_invalid_json_are_explicit() {
        let error = execute(
            python("import sys; sys.stderr.write('ExpiredToken\\x1b'); sys.exit(1)"),
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        assert!(error.contains("ExpiredToken"));
        assert!(!error.contains('\x1b'));
        assert!(execute(python("print('not json')"), Duration::from_secs(5))
            .await
            .unwrap_err()
            .contains("valid JSON"));
    }
    #[tokio::test]
    async fn output_and_error_streams_are_bounded_without_deadlock() {
        for script in [
            "import sys; sys.stdout.write('x'*1100000)",
            "import sys; sys.stderr.write('x'*70000)",
        ] {
            assert!(execute(python(script), Duration::from_secs(5))
                .await
                .unwrap_err()
                .contains("limit"));
        }
    }
    #[tokio::test]
    async fn times_out_and_reports_missing_cli() {
        assert!(execute(
            python("import time; time.sleep(60)"),
            Duration::from_millis(50)
        )
        .await
        .unwrap_err()
        .contains("timed out"));
        assert!(execute(
            Command::new("rusticity-no-such-executable"),
            Duration::from_secs(1)
        )
        .await
        .unwrap_err()
        .contains("not installed"));
    }
}

#[cfg(all(test, target_os = "linux"))]
mod cancellation_tests {
    use super::*;
    use std::{io::Read, net::TcpListener};

    #[tokio::test]
    async fn aborting_runner_terminates_the_actual_child_process() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let script = format!("import os,socket,time; s=socket.create_connection(('127.0.0.1',{})); s.sendall(str(os.getpid()).encode()); s.close(); time.sleep(60)",address.port());
        let mut command = Command::new("python3");
        command.args(["-c", &script]);
        let task = tokio::spawn(execute(command, Duration::from_secs(30)));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let pid = loop {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut pid = String::new();
                stream.read_to_string(&mut pid).unwrap();
                break pid.parse::<u32>().unwrap();
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "child did not start"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let path = format!("/proc/{pid}/stat");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            match std::fs::read_to_string(&path) {
                Err(_) => break,
                Ok(stat) if stat.split_whitespace().nth(2) == Some("Z") => break,
                _ => {}
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "CLI child remained alive after abort"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

#[cfg(test)]
mod installed_cli_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Instant,
    };

    #[tokio::test]
    #[ignore = "requires AWS CLI on PATH; localhost-only fixture"]
    async fn installed_cli_runner_returns_pretty_json_from_local_fixture() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("{error}"),
                }
                assert!(
                    Instant::now() < deadline,
                    "CLI did not contact loopback fixture"
                );
                thread::sleep(Duration::from_millis(10));
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = vec![];
            loop {
                let mut bytes = [0u8; 4096];
                let n = stream.read(&mut bytes).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&bytes[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..end]);
                    let len = header
                        .lines()
                        .find_map(|line| {
                            let (k, v) = line.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.contains("Action=GetTopicAttributes"));
            assert!(request.contains("TopicArn=arn%3Aaws%3Asns%3Aus-east-1%3A123456789012%3Atest"));
            let body="<GetTopicAttributesResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><GetTopicAttributesResult><Attributes><entry><key>DisplayName</key><value>Fixture topic</value></entry></Attributes></GetTopicAttributesResult><ResponseMetadata><RequestId>test</RequestId></ResponseMetadata></GetTopicAttributesResponse>";
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        // Build the same trusted argv used by the copy and execution bridge.
        // Dummy environment belongs only to this subprocess, not the user's profile.
        let mut command = Command::new("aws");
        command
            .args([
                "sns",
                "get-topic-attributes",
                "--cli-input-json",
                "{\"TopicArn\":\"arn:aws:sns:us-east-1:123456789012:test\"}",
                "--region=us-east-1",
                "--output",
                "json",
                "--no-cli-pager",
                "--no-cli-auto-prompt",
                "--endpoint-url",
                &format!("http://{address}"),
            ])
            .env("AWS_ACCESS_KEY_ID", "test")
            .env("AWS_SECRET_ACCESS_KEY", "test")
            .env("AWS_CONFIG_FILE", "/dev/null")
            .env("AWS_SHARED_CREDENTIALS_FILE", "/dev/null")
            .env("AWS_EC2_METADATA_DISABLED", "true")
            .env("AWS_MAX_ATTEMPTS", "1")
            .env("AWS_PAGER", "")
            .env("NO_PROXY", "127.0.0.1")
            .env_remove("AWS_PROFILE")
            .env_remove("AWS_SESSION_TOKEN");
        let result = execute(command, Duration::from_secs(15)).await;
        server.join().unwrap();
        let text = result.unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).unwrap()["Attributes"]["DisplayName"],
            "Fixture topic"
        );
        assert!(text.contains("\n  \"Attributes\""));
    }
}
