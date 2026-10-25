//! Explicit real llama-server smoke. No user model, live account or GPU required.
#![cfg(windows)]
use blink_verdict::{
    config::{Config, Provider},
    decision::Verdict,
    provider::{decide, Observation},
};

#[test]
#[ignore = "release-only: requires the pinned llama-server fixture started by llama-smoke.ps1"]
fn real_llama_server_schema_and_authentication() {
    let endpoint =
        std::env::var("BLINK_LLAMA_TEST_ENDPOINT").expect("run scripts/release/llama-smoke.ps1");
    let config = Config {
        provider: Provider::LlamaCpp,
        endpoint,
        model: "blink-release".into(),
        vision: false,
        timeout_seconds: 30,
        output_tokens: 128,
        prompt: "Choose an offered answer. If unsure, abstain.".into(),
        ..Config::default()
    };
    for _ in 0..2 {
        let answer = decide(
            &config,
            "synthetic-release-key",
            Observation::Text("Synthetic screen: there are two options.".into()),
        )
        .unwrap();
        assert!(matches!(
            answer.verdict,
            Verdict::Choice(0) | Verdict::Choice(1) | Verdict::Uncertain
        ));
    }
    let error = decide(&config, "", Observation::Text("synthetic".into())).unwrap_err();
    assert!(
        error.contains("401"),
        "missing authentication was not rejected: {error}"
    );
}
