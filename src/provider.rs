use crate::{
    config::{Config, Provider},
    decision::{instruction, parse_verdict, Verdict},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{blocking::Client, Url};
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

pub enum Observation {
    Image(Vec<u8>),
    Text(String),
}

pub fn validate_endpoint(endpoint: &str) -> Result<(), String> {
    let url = Url::parse(endpoint.trim()).map_err(|_| "API 地址无效".to_string())?;
    let host = url.host_str().unwrap_or("").trim_matches(['[', ']']);
    let loopback = host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("API 地址须使用 HTTPS；仅本机回环地址可使用 HTTP".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("API 地址不能包含账号、密码、查询参数或片段；密钥请单独填写".into());
    }
    Ok(())
}

fn request_parts(config: &Config, observation: &Observation) -> (String, Value) {
    let base = config.endpoint.trim().trim_end_matches('/');
    let task = instruction(config);
    let image = match observation {
        Observation::Image(bytes) => Some(STANDARD.encode(bytes)),
        _ => None,
    };
    let text = match observation {
        Observation::Text(s) => format!("Screen OCR observations:\n{s}"),
        _ => "Inspect the attached screen image.".into(),
    };
    match config.provider {
        Provider::OpenAi => {
            let mut content = vec![json!({"type":"text", "text":text})];
            if let Some(b64) = image {
                content.push(json!({"type":"image_url", "image_url":{"url":format!("data:image/png;base64,{b64}")}}));
            }
            (
                format!("{base}/chat/completions"),
                json!({"model":config.model,"messages":[{"role":"system","content":task},{"role":"user","content":content}],"max_tokens":config.output_tokens}),
            )
        }
        Provider::OpenAiResponses => {
            let mut content = vec![json!({"type":"input_text", "text":text})];
            if let Some(b64) = image {
                content.push(json!({"type":"input_image", "image_url":format!("data:image/png;base64,{b64}")}));
            }
            (
                format!("{base}/responses"),
                json!({"model":config.model,"instructions":task,"input":[{"role":"user","content":content}],"max_output_tokens":config.output_tokens,"store":false}),
            )
        }
        Provider::Anthropic => {
            let mut content = vec![json!({"type":"text", "text":text})];
            if let Some(b64) = image {
                content.push(json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":b64}}));
            }
            (
                format!("{base}/messages"),
                json!({"model":config.model,"system":task,"messages":[{"role":"user","content":content}],"max_tokens":config.output_tokens}),
            )
        }
        Provider::Gemini => {
            let mut parts = vec![json!({"text":text})];
            if let Some(b64) = image {
                parts.push(json!({"inlineData":{"mimeType":"image/png","data":b64}}));
            }
            (
                format!("{base}/models/{}:generateContent", config.model),
                json!({"systemInstruction":{"parts":[{"text":task}]},"contents":[{"role":"user","parts":parts}],"generationConfig":{"maxOutputTokens":config.output_tokens,"responseMimeType":"application/json"}}),
            )
        }
        Provider::Ollama => {
            let mut message = json!({"role":"user","content":text});
            if let Some(b64) = image {
                message["images"] = json!([b64]);
            }
            (
                format!("{base}/api/chat"),
                json!({"model":config.model,"messages":[{"role":"system","content":task},message],"stream":false,"format":"json","keep_alive":0,"options":{"num_predict":config.output_tokens}}),
            )
        }
    }
}

fn extract_text(provider: Provider, value: &Value) -> Result<String, String> {
    let texts: Vec<&str> = match provider {
        Provider::OpenAi => value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .into_iter()
            .collect(),
        Provider::Ollama => value
            .pointer("/message/content")
            .and_then(Value::as_str)
            .into_iter()
            .collect(),
        Provider::Anthropic => value["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v["type"] == "text")
            .filter_map(|v| v["text"].as_str())
            .collect(),
        Provider::Gemini => value
            .pointer("/candidates/0/content/parts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|v| v["thought"] != true)
            .filter_map(|v| v["text"].as_str())
            .collect(),
        Provider::OpenAiResponses => value["output"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v["type"] == "message")
            .filter_map(|v| v["content"].as_array())
            .flatten()
            .filter(|v| v["type"] == "output_text")
            .filter_map(|v| v["text"].as_str())
            .collect(),
    };
    if texts.is_empty() {
        Err("模型没有返回可用文字（可能拒绝请求、模型不支持图片或输出额度不足）".into())
    } else {
        Ok(texts.join(""))
    }
}

pub fn decide(config: &Config, key: &str, observation: Observation) -> Result<Verdict, String> {
    config.validate()?;
    if let Observation::Text(text) = &observation {
        if text.trim().is_empty() {
            return Err("OCR 未识别到文字，请改用视觉模型".into());
        }
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(config.timeout_seconds.into()))
        .connect_timeout(Duration::from_secs(8))
        // Do not forward screenshots or credentials to an unexpected redirect target.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法创建网络客户端".to_string())?;
    let (url, body) = request_parts(config, &observation);
    let mut request = client.post(url).json(&body);
    if !key.is_empty() {
        request = match config.provider {
            Provider::Anthropic => request.header("x-api-key", key),
            Provider::Gemini => request.header("x-goog-api-key", key),
            _ => request.bearer_auth(key),
        };
    }
    if config.provider == Provider::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }
    let response = request.send().map_err(|e| {
        if e.is_timeout() {
            "模型请求超时"
        } else {
            "模型连接失败，请检查网络、地址与证书"
        }
        .to_string()
    })?;
    if !response.status().is_success() {
        return Err(format!(
            "模型服务返回 HTTP {}；请检查地址、密钥、模型名称与额度",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "模型响应读取失败".to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("模型响应过大".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "模型服务未返回有效 JSON".to_string())?;
    parse_verdict(
        &extract_text(config.provider, &value)?,
        config.options.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_wire_formats_and_response_selection() {
        let mut c = Config::default();
        for p in [
            Provider::OpenAi,
            Provider::OpenAiResponses,
            Provider::Anthropic,
            Provider::Gemini,
            Provider::Ollama,
        ] {
            c.provider = p;
            c.endpoint = p.defaults().0.into();
            let (url, body) = request_parts(&c, &Observation::Image(vec![1, 2, 3]));
            assert!(url.starts_with(&c.endpoint));
            assert!(body.to_string().contains("AQID"));
            let (_, text_body) = request_parts(&c, &Observation::Text("screen words".into()));
            assert!(text_body.to_string().contains("screen words"));
        }
        assert_eq!(extract_text(Provider::OpenAiResponses, &json!({"output":[{"type":"reasoning"},{"type":"message","content":[{"type":"output_text","text":"{\"choice\":null}"}]}]})).unwrap(), "{\"choice\":null}");
        assert!(extract_text(Provider::OpenAi, &json!({"choices":[]})).is_err());
    }
    #[test]
    fn endpoints_do_not_leak_credentials_over_plain_http() {
        assert!(validate_endpoint("http://127.0.0.1:11434").is_ok());
        assert!(validate_endpoint("https://example.com/v1").is_ok());
        assert!(validate_endpoint("http://example.com/v1").is_err());
        assert!(validate_endpoint("https://example.com/v1?key=secret").is_err());
        assert!(validate_endpoint("https://user:secret@example.com/v1").is_err());
    }
}
