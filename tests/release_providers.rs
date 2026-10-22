//! Explicit release-only network regressions. Never contacts a real model service.
use blink_verdict::{
    config::{Config, Provider},
    decision::Verdict,
    provider::{decide, Observation},
};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

fn request(stream: &mut TcpStream) -> (String, Value) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = Vec::new();
    let end = loop {
        let mut part = [0; 1024];
        let count = stream.read(&mut part).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&part[..count]);
        if let Some(index) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            break index + 4;
        }
        assert!(bytes.len() < 100000);
    };
    let header = String::from_utf8(bytes[..end].to_vec()).unwrap();
    let length: usize = header
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|n| n.trim().parse().unwrap())
        })
        .unwrap();
    while bytes.len() < end + length {
        let mut part = [0; 4096];
        let n = stream.read(&mut part).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&part[..n]);
    }
    (
        header,
        serde_json::from_slice(&bytes[end..end + length]).unwrap(),
    )
}

fn server(
    handler: impl FnOnce(&mut TcpStream) + Send + 'static,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        let start = Instant::now();
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    handler(&mut stream);
                    break;
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        && start.elapsed() < Duration::from_secs(5) =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(e) => panic!("mock service did not receive request: {e}"),
            }
        }
    });
    (address, worker)
}

fn reply(stream: &mut TcpStream, status: &str, body: &str, extra: &str) {
    let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}", body.len());
}

#[test]
#[ignore = "release-only: exercises the real HTTP transport for every adapter"]
fn all_adapters_send_observations_and_map_results() {
    for provider in [
        Provider::OpenAi,
        Provider::OpenAiResponses,
        Provider::Anthropic,
        Provider::Gemini,
        Provider::Ollama,
    ] {
        for vision in [false, true] {
            let (endpoint, worker) = server(move |stream| {
                let (header, body) = request(stream);
                let lower = header.to_ascii_lowercase();
                assert!(lower.contains("release-test-key"));
                match provider {
                    Provider::Anthropic => assert!(
                        lower.contains("x-api-key:") && lower.contains("anthropic-version:")
                    ),
                    Provider::Gemini => assert!(lower.contains("x-goog-api-key:")),
                    _ => assert!(lower.contains("authorization: bearer")),
                }
                assert!(body.to_string().contains(if vision {
                    "AQID"
                } else {
                    "visible screen text"
                }));
                let answer = "{\"choice\":\"option_2\"}";
                let response = match provider {
                    Provider::OpenAi => json!({"choices":[{"message":{"content":answer}}]}),
                    Provider::OpenAiResponses => {
                        json!({"output":[{"type":"message","content":[{"type":"output_text","text":answer}]}]})
                    }
                    Provider::Anthropic => json!({"content":[{"type":"text","text":answer}]}),
                    Provider::Gemini => {
                        json!({"candidates":[{"content":{"parts":[{"text":answer}]}}]})
                    }
                    Provider::Ollama => {
                        assert_eq!(body["keep_alive"], 0);
                        json!({"message":{"content":answer}})
                    }
                };
                reply(stream, "200 OK", &response.to_string(), "");
            });
            let config = Config {
                provider,
                endpoint,
                vision,
                ..Config::default()
            };
            let input = if vision {
                Observation::Image(vec![1, 2, 3])
            } else {
                Observation::Text("visible screen text".into())
            };
            let result = decide(&config, "release-test-key", input);
            worker.join().unwrap();
            assert_eq!(result.unwrap(), Verdict::Choice(1));
        }
    }
}

#[test]
#[ignore = "release-only: failure and redirect handling"]
fn http_errors_and_redirects_do_not_expose_server_bodies_or_follow_locations() {
    for status in ["401 Unauthorized", "429 Too Many Requests", "302 Found"] {
        let (endpoint, worker) = server(move |stream| {
            request(stream);
            reply(
                stream,
                status,
                "secret-untrusted-server-error",
                "Location: http://127.0.0.1:1/do-not-follow\r\n",
            );
        });
        let config = Config {
            endpoint,
            ..Config::default()
        };
        let error = decide(&config, "", Observation::Text("screen".into())).unwrap_err();
        worker.join().unwrap();
        assert!(error.contains(&status[..3]));
        assert!(!error.contains("secret-untrusted"));
    }
}

#[test]
#[ignore = "release-only: timeout and malformed model output"]
fn timeout_and_unknown_choice_are_failures() {
    let (endpoint, worker) = server(|stream| {
        request(stream);
        thread::sleep(Duration::from_millis(1400));
        reply(stream, "200 OK", "{}", "");
    });
    let config = Config {
        endpoint,
        timeout_seconds: 1,
        ..Config::default()
    };
    assert!(decide(&config, "", Observation::Text("screen".into()))
        .unwrap_err()
        .contains("超时"));
    worker.join().unwrap();
    let (endpoint, worker) = server(|stream| {
        request(stream);
        reply(
            stream,
            "200 OK",
            &json!({"choices":[{"message":{"content":"{\"choice\":\"option_99\"}"}}]}).to_string(),
            "",
        );
    });
    let config = Config {
        endpoint,
        ..Config::default()
    };
    assert!(decide(&config, "", Observation::Text("screen".into())).is_err());
    worker.join().unwrap();
}
