//! Headless bridge to the per-user AI provider configured in easyPDF Web.

use std::time::Duration;

use serde_json::{Value, json};

use crate::{Args, Automation, Result, ToolError, failed};

const MAX_AI_TEXT_BYTES: usize = 500_000;
const MAX_AI_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

impl Automation {
    pub(super) fn ai_document(&mut self, args: &Args<'_>, task: &str) -> Result<Value> {
        let extracted = self.text_extract(args)?;
        let mut text = String::new();
        let pages = extracted.get("pages").and_then(Value::as_array).ok_or_else(|| failed("text extraction returned no pages"))?;
        for page in pages {
            let part = page.get("text").and_then(Value::as_str).unwrap_or_default();
            let marker = if task == "ask" { format!("[Page {}]\n", page.get("page").and_then(Value::as_u64).unwrap_or(0)) } else { String::new() };
            let extra = part.len().saturating_add(marker.len()).saturating_add(2);
            if text.len().saturating_add(extra) > MAX_AI_TEXT_BYTES {
                return Err(ToolError::Failed(format!("document text exceeds {MAX_AI_TEXT_BYTES} bytes; pass fewer pages")));
            }
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(&marker);
            text.push_str(part);
        }
        if text.trim().is_empty() {
            return Err(ToolError::Failed("the selected pages contain no extractable text; run OCR first".into()));
        }

        let base = std::env::var("EASYPDF_WEB_URL")
            .map_err(|_| ToolError::Failed("set EASYPDF_WEB_URL to the easyPDF Web installation (for example http://localhost:8282)".into()))?;
        let parsed = url::Url::parse(&base).map_err(|_| ToolError::Failed("EASYPDF_WEB_URL is not a valid URL".into()))?;
        let local_http = parsed.scheme() == "http" && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1"));
        if !((parsed.scheme() == "https" && parsed.host_str().is_some()) || local_http)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(ToolError::Failed("EASYPDF_WEB_URL needs HTTPS or an exact localhost host, without credentials, query or fragment".into()));
        }
        let token = std::env::var("EASYPDF_WEB_TOKEN")
            .map_err(|_| ToolError::Failed("set EASYPDF_WEB_TOKEN to a desktop access token from easyPDF Web".into()))?;
        if token.trim().is_empty() {
            return Err(ToolError::Failed("EASYPDF_WEB_TOKEN is empty".into()));
        }
        let url = format!("{}/app-api.php?action=ai-run", base.trim_end_matches('/'));
        let body = json!({
            "task": task,
            "text": text,
            "question": args.opt_str("question")?.unwrap_or_default(),
            "language": args.opt_str("language")?.unwrap_or_default(),
        });
        let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().new_agent();
        let mut response = agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .config()
            .http_status_as_error(false)
            .build()
            .send(body.to_string())
            .map_err(|error| failed(format!("easyPDF Web is not reachable: {error}")))?;
        let status = response.status().as_u16();
        let raw = response
            .body_mut()
            .with_config()
            .limit(MAX_AI_RESPONSE_BYTES)
            .read_to_string()
            .map_err(|error| failed(format!("could not read the easyPDF Web response: {error}")))?;
        let value: Value = serde_json::from_str(&raw).map_err(|error| failed(format!("easyPDF Web returned invalid JSON: {error}")))?;
        if !(200..300).contains(&status) {
            let message = value.get("error").and_then(Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_owned);
            return Err(ToolError::Failed(message));
        }
        let result = value.get("result").and_then(Value::as_str).ok_or_else(|| failed("easyPDF Web returned no AI result"))?;
        Ok(json!({ "task": task, "result": result }))
    }
}
