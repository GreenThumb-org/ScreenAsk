use crate::config::Config;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiApiError>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    parts: Option<Vec<Part>>,
}

#[derive(Debug, Deserialize)]
struct Part {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiApiError {
    code: Option<u16>,
    message: Option<String>,
    status: Option<String>,
}

pub fn ask_gemini(cfg: &Config, jpeg_bytes: &[u8], pdf_context: &str) -> Result<String, String> {
    let api_key = cfg
        .effective_api_key()
        .ok_or_else(|| "No API key set (tray icon > Open ScreenAsk).".to_string())?;

    let mut models_to_try = vec![cfg.model.trim().to_string()];
    let fb = cfg.fallback_model.trim();
    if !fb.is_empty() && !models_to_try.iter().any(|m| m == fb) {
        models_to_try.push(fb.to_string());
    }

    let b64_image = STANDARD.encode(jpeg_bytes);
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
    parts.push(json!({
        "text": cfg.prompt
    }));

    let payload = json!({
        "contents": [
            {
                "parts": parts
            }
        ]
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut last_err = String::new();

    for (idx, model) in models_to_try.iter().enumerate() {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            model
        );

        let resp_res = client
            .post(&url)
            .header("x-goog-api-key", &api_key)
            .json(&payload)
            .send();

        match resp_res {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();

                if status.is_success() {
                    let parsed: GeminiResponse = serde_json::from_str(&body)
                        .map_err(|e| format!("Failed to parse Gemini response: {e}"))?;

                    let text = extract_answer(&parsed);
                    if idx > 0 {
                        return Ok(format!("[Fallback]\n{text}"));
                    }
                    return Ok(text);
                }

                let err_msg = format_api_error(status.as_u16(), &body);
                let retryable = is_retryable_error(Some(status.as_u16()), &body);
                last_err = err_msg;

                if idx < models_to_try.len() - 1 && retryable {
                    continue;
                }
                return Err(last_err);
            }
            Err(e) => {
                last_err = format!("Request failed: {e}");
                let retryable = e.is_timeout() || e.is_connect();
                if idx < models_to_try.len() - 1 && retryable {
                    continue;
                }
                return Err(last_err);
            }
        }
    }

    Err(if last_err.is_empty() {
        "Unknown Gemini API error".to_string()
    } else {
        last_err
    })
}

fn extract_answer(resp: &GeminiResponse) -> String {
    let mut out = String::new();
    if let Some(candidates) = &resp.candidates {
        if let Some(first) = candidates.first() {
            if let Some(content) = &first.content {
                if let Some(parts) = &content.parts {
                    for part in parts {
                        if let Some(t) = &part.text {
                            out.push_str(t);
                        }
                    }
                }
            }
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        "(empty answer)".to_string()
    } else {
        trimmed.to_string()
    }
}

fn format_api_error(status: u16, body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<GeminiResponse>(body) {
        if let Some(err) = parsed.error {
            let code = err.code.unwrap_or(status);
            let st = err.status.unwrap_or_default();
            let msg = err.message.unwrap_or_else(|| body.to_string());
            if st.is_empty() {
                return format!("{code}: {msg}");
            }
            return format!("{code} {st}: {msg}");
        }
    }
    format!("HTTP {status}: {body}")
}

fn is_retryable_error(status: Option<u16>, body: &str) -> bool {
    if matches!(status, Some(429 | 503 | 500 | 404)) {
        return true;
    }
    let upper = body.to_uppercase();
    upper.contains("RESOURCE_EXHAUSTED")
        || upper.contains("UNAVAILABLE")
        || upper.contains("OVERLOADED")
        || body.contains("429")
        || body.contains("503")
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
        assert!(is_retryable_error(Some(404), ""));
        assert!(is_retryable_error(Some(400), "RESOURCE_EXHAUSTED"));
        assert!(!is_retryable_error(Some(401), "API_KEY_INVALID"));
    }
}
