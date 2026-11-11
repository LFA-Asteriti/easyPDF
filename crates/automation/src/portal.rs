//! Headless access to authenticated organization sharing in easyPDF Web.

use std::collections::BTreeSet;
use std::time::Duration;

use base64::Engine as _;
use serde_json::{Value, json};

use crate::{Args, Automation, Result, ToolError, failed};

const MAX_PORTAL_JSON_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SHARED_PDF_BYTES: u64 = 210_000_000;

fn portal_connection() -> Result<(String, String)> {
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
    Ok((base.trim_end_matches('/').to_string(), token))
}

fn portal_json(action: &str, body: Option<Value>) -> Result<Value> {
    let (base, token) = portal_connection()?;
    let url = format!("{base}/app-api.php?action={action}");
    let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().new_agent();
    let mut response = if let Some(body) = body {
        agent
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .config()
            .http_status_as_error(false)
            .build()
            .send(body.to_string())
    } else {
        agent.get(&url).header("Authorization", &format!("Bearer {token}")).config().http_status_as_error(false).build().call()
    }
    .map_err(|error| failed(format!("easyPDF Web is not reachable: {error}")))?;
    let status = response.status().as_u16();
    let raw = response
        .body_mut()
        .with_config()
        .limit(MAX_PORTAL_JSON_BYTES)
        .read_to_string()
        .map_err(|error| failed(format!("could not read the easyPDF Web response: {error}")))?;
    let value: Value = serde_json::from_str(&raw).map_err(|error| failed(format!("easyPDF Web returned invalid JSON: {error}")))?;
    if !(200..300).contains(&status) {
        let message = value.get("error").and_then(Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_owned);
        return Err(ToolError::Failed(message));
    }
    Ok(value)
}

impl Automation {
    pub(super) fn portal_share_document(&mut self, args: &Args<'_>) -> Result<Value> {
        let id = self.doc(args)?.id;
        let name = self.doc(args)?.name.clone();
        let recipient_ids: BTreeSet<u64> = args
            .ints("recipient_ids")?
            .into_iter()
            .map(|value| u64::try_from(value).map_err(|_| ToolError::InvalidArgs("recipient_ids must contain positive ids".into())))
            .collect::<Result<_>>()?;
        if recipient_ids.is_empty() || recipient_ids.len() > 100 {
            return Err(ToolError::InvalidArgs("recipient_ids must contain 1 to 100 unique ids".into()));
        }
        let bytes = self.session.save_bytes(id).map_err(failed)?;
        let value = portal_json(
            "document-share",
            Some(json!({
                "name": name,
                "document": base64::engine::general_purpose::STANDARD.encode(bytes.as_ref()),
                "recipient_ids": recipient_ids,
            })),
        )?;
        Ok(json!({ "message": value.get("message").and_then(Value::as_str).unwrap_or("document shared") }))
    }

    pub(super) fn portal_snapshot_list(&self, key: &str) -> Result<Value> {
        let value = portal_json("snapshot", None)?;
        let list = value
            .get("snapshot")
            .and_then(|snapshot| snapshot.get(key))
            .and_then(Value::as_array)
            .ok_or_else(|| failed(format!("easyPDF Web snapshot has no {key}")))?;
        Ok(json!({ "kind": key, "items": list }))
    }

    pub(super) fn portal_shared_open(&self, args: &Args<'_>) -> Result<Value> {
        let id = u64::try_from(args.int("id")?).map_err(|_| ToolError::InvalidArgs("id must be positive".into()))?;
        let (base, token) = portal_connection()?;
        let url = format!("{base}/app-api.php?action=document-open&id={id}");
        let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().new_agent();
        let mut response = agent
            .get(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|error| failed(format!("easyPDF Web is not reachable: {error}")))?;
        let status = response.status().as_u16();
        let bytes = response
            .body_mut()
            .with_config()
            .limit(MAX_SHARED_PDF_BYTES)
            .read_to_vec()
            .map_err(|error| failed(format!("could not read the shared document: {error}")))?;
        if !(200..300).contains(&status) {
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| failed(format!("HTTP {status}")))?;
            let message = value.get("error").and_then(Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_owned);
            return Err(ToolError::Failed(message));
        }
        if !bytes.starts_with(b"%PDF-") {
            return Err(ToolError::Failed("easyPDF Web returned a file that is not a PDF".into()));
        }
        let path = self.write_output(args.str("out")?, &bytes)?;
        Ok(json!({ "id": id, "path": path.to_string_lossy(), "bytes": bytes.len() }))
    }

    pub(super) fn portal_notification_read(&self, args: &Args<'_>) -> Result<Value> {
        let id = u64::try_from(args.int("id")?).map_err(|_| ToolError::InvalidArgs("id must be positive".into()))?;
        let value = portal_json("notification-read", Some(json!({ "id": id })))?;
        Ok(json!({ "id": id, "message": value.get("message").and_then(Value::as_str).unwrap_or("notification read") }))
    }
}
