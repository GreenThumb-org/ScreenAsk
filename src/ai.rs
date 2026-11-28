use crate::config::{ApiProvider, Config, DEFAULT_OPENAI_BASE_URL};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

pub fn ask_ai(cfg: &Config, jpeg_bytes: &[u8], pdf_context: &str) -> Result<String, String> {
    let api_key = match cfg.effective_api_key() {
        Some(k) => k,
        None if cfg.is_local_openai_endpoint() => String::new(),
        None => {
            return Err(format!(
                "No API key set for {} (tray icon > Open ScreenAsk).",
                cfg.provider.label()
            ));
        }
    };

    let mut models_to_try = vec![cfg.model.trim().to_string()];
    let fb = cfg.fallback_model.trim();
    if !fb.is_empty() && !models_to_try.iter().any(|m| m == fb) {
        models_to_try.push(fb.to_string());
    }

    let b64_image = STANDARD.encode(jpeg_bytes);
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut last_err = String::new();

    for (idx, model) in models_to_try.iter().enumerate() {
        let attempt = match cfg.provider {
            ApiProvider::Gemini => {
                call_gemini(&client, &api_key, model, &cfg.prompt, &b64_image, pdf_context)
            }
            ApiProvider::Claude => {
                call_claude(&client, &api_key, model, &cfg.prompt, &b64_image, pdf_context)
            }
            ApiProvider::OpenAi => call_openai(
                &client,
                &api_key,
                &cfg.api_base_url,
                model,
                &cfg.prompt,
                &b64_image,
                pdf_context,
            ),
        };

        match attempt {
            Ok(text) => {
                let answer = if text.trim().is_empty() {
                    "(empty answer)".to_string()
                } else {
                    text.trim().to_string()
                };
                if idx > 0 {
                    return Ok(format!("[Fallback]\n{answer}"));
                }
                return Ok(answer);
            }
            Err((err_msg, retryable)) => {
                last_err = err_msg;
                if idx < models_to_try.len() - 1 && retryable {
                    continue;
                }
                return Err(last_err);
            }
        }
    }

    Err(if last_err.is_empty() {
        "Unknown AI API error".to_string()
    } else {
        last_err
    })
}

// --- Google Gemini ---

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GenericApiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

fn call_gemini(
    client: &Client,
    api_key: &str,
    model: &str,
    prompt: &str,
    b64_image: &str,
    pdf_context: &str,
) -> Result<String, (String, bool)> {
    let mut parts = Vec::new();
    if !pdf_context.trim().is_empty() {
        parts.push(json!({
            "text": format!("Additional context / reference documents from folder:\n{pdf_context}\n\n")
        }));
    }
    parts.push(json!({
        "inline_data": {
            "mime_type": "image/jpeg",
            "data": b64_image
        }
    }));
    parts.push(json!({ "text": prompt }));

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .json(&json!({ "contents": [{ "parts": parts }] }))
        .send()
        .map_err(|e| (format!("Request failed: {e}"), e.is_timeout() || e.is_connect()))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();

    if !status.is_success() {
        return Err((
            format_generic_error(status.as_u16(), &body),
            is_retryable_error(Some(status.as_u16()), &body),
        ));
    }

    let parsed: GeminiResponse = serde_json::from_str(&body)
        .map_err(|e| (format!("Failed to parse Gemini response: {e}"), false))?;

    let mut out = String::new();
    if let Some(cands) = parsed.candidates {
        if let Some(first) = cands.first() {
            if let Some(content) = &first.content {
                if let Some(parts) = &content.parts {
                    for p in parts {
                        if let Some(t) = &p.text {
                            out.push_str(t);
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

// --- Anthropic Claude ---

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Option<Vec<ClaudeBlock>>,
}

#[derive(Debug, Deserialize)]
struct ClaudeBlock {
    #[serde(rename = "type")]
    kind: Option<String>,
    text: Option<String>,
}

fn call_claude(
    client: &Client,
    api_key: &str,
    model: &str,
    prompt: &str,
    b64_image: &str,
    pdf_context: &str,
) -> Result<String, (String, bool)> {
    let mut blocks = Vec::new();
    if !pdf_context.trim().is_empty() {
        blocks.push(json!({
            "type": "text",
            "text": format!("Additional context / reference documents from folder:\n{pdf_context}\n\n")
        }));
    }
    blocks.push(json!({
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": "image/jpeg",
            "data": b64_image
        }
    }));
    blocks.push(json!({
        "type": "text",
        "text": prompt
    }));

    let payload = json!({
        "model": model,
        "max_tokens": 1024,
        "messages": [
            {
                "role": "user",
                "content": blocks
            }
        ]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .map_err(|e| (format!("Request failed: {e}"), e.is_timeout() || e.is_connect()))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();

    if !status.is_success() {
        return Err((
            format_generic_error(status.as_u16(), &body),
            is_retryable_error(Some(status.as_u16()), &body),
        ));
    }

    let parsed: ClaudeResponse = serde_json::from_str(&body)
        .map_err(|e| (format!("Failed to parse Claude response: {e}"), false))?;

    let mut out = String::new();
    if let Some(content) = parsed.content {
        for block in content {
            if block.kind.as_deref() == Some("text") {
                if let Some(t) = block.text {
                    out.push_str(&t);
                }
            }
        }
    }
    Ok(out)
}

// --- OpenAI / OpenAI-Compatible (OpenAI, OpenRouter, Groq, Ollama, LM Studio) ---

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Option<Vec<OpenAiChoice>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<serde_json::Value>,
}

fn call_openai(
    client: &Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    prompt: &str,
    b64_image: &str,
    pdf_context: &str,
) -> Result<String, (String, bool)> {
    let base = {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            DEFAULT_OPENAI_BASE_URL
        } else {
            trimmed
        }
    };
    let url = format!("{base}/chat/completions");

    let mut content_parts = Vec::new();
    if !pdf_context.trim().is_empty() {
        content_parts.push(json!({
            "type": "text",
            "text": format!("Additional context / reference documents from folder:\n{pdf_context}\n\n")
        }));
    }
    content_parts.push(json!({
        "type": "image_url",
        "image_url": {
            "url": format!("data:image/jpeg;base64,{b64_image}")
        }
    }));
    content_parts.push(json!({
        "type": "text",
        "text": prompt
    }));

    let payload = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": content_parts
            }
        ]
    });

    let mut req = client.post(&url).json(&payload);
    if !api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
    }

    let resp = req
        .send()
        .map_err(|e| (format!("Request failed: {e}"), e.is_timeout() || e.is_connect()))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();

    if !status.is_success() {
        return Err((
            format_generic_error(status.as_u16(), &body),
            is_retryable_error(Some(status.as_u16()), &body),
        ));
    }

    let parsed: OpenAiResponse = serde_json::from_str(&body)
        .map_err(|e| (format!("Failed to parse OpenAI response: {e}"), false))?;

    let mut out = String::new();
    if let Some(choices) = parsed.choices {
        if let Some(first) = choices.first() {
            if let Some(msg) = &first.message {
                if let Some(val) = &msg.content {
                    if let Some(s) = val.as_str() {
                        out.push_str(s);
                    } else if let Some(arr) = val.as_array() {
                        for item in arr {
                            if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                                out.push_str(t);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

// --- Shared Error Helpers ---

#[derive(Debug, Deserialize)]
struct GenericErrorEnvelope {
    error: Option<GenericApiError>,
}

#[derive(Debug, Deserialize)]
struct GenericApiError {
    code: Option<serde_json::Value>,
    message: Option<String>,
    status: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

fn format_generic_error(status: u16, body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<GenericErrorEnvelope>(body) {
        if let Some(err) = parsed.error {
            let label = err.status.or(err.kind).unwrap_or_default();
            let msg = err.message.unwrap_or_else(|| body.to_string());
            let code_str = err
                .code
                .map(|c| c.to_string().trim_matches('"').to_string())
                .unwrap_or_else(|| status.to_string());
            if label.is_empty() {
                return format!("{code_str}: {msg}");
            }
            return format!("{code_str} {label}: {msg}");
        }
    }
    if let Ok(parsed) = serde_json::from_str::<GeminiResponse>(body) {
        if let Some(err) = parsed.error {
            let msg = err.message.unwrap_or_else(|| body.to_string());
            return format!("{status}: {msg}");
        }
    }
    format!("HTTP {status}: {body}")
}

fn is_retryable_error(status: Option<u16>, body: &str) -> bool {
    if matches!(status, Some(429 | 500 | 502 | 503 | 504 | 529 | 404)) {
        return true;
    }
    let upper = body.to_uppercase();
    upper.contains("RESOURCE_EXHAUSTED")
        || upper.contains("UNAVAILABLE")
        || upper.contains("OVERLOADED")
        || upper.contains("RATE_LIMIT")
        || body.contains("429")
        || body.contains("503")
        || body.contains("529")
        || body.contains("500")
        || body.contains("404")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_retryable_errors() {
        assert!(is_retryable_error(Some(429), ""));
        assert!(is_retryable_error(Some(503), ""));
        assert!(is_retryable_error(Some(529), "overloaded_error"));
        assert!(is_retryable_error(Some(404), ""));
        assert!(is_retryable_error(Some(400), "RESOURCE_EXHAUSTED"));
        assert!(!is_retryable_error(Some(401), "API_KEY_INVALID"));
    }
}
