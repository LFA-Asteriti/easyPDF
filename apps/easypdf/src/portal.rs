//! Native bridge to the authenticated easyPDF Web API.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use easypdf_ui_egui::connected::{AiTask, Command, CommandFn, Event, EventQueue, Snapshot};

pub(crate) fn command_fn(events: EventQueue, ctx: egui::Context) -> CommandFn {
    let base = Arc::new(Mutex::new(String::new()));
    let token = Arc::new(Mutex::new(String::new()));
    Box::new(move |command| {
        let events = events.clone();
        let ctx = ctx.clone();
        let base = base.clone();
        let token = token.clone();
        std::thread::spawn(move || {
            let event = execute(command, &base, &token).unwrap_or_else(Event::Error);
            match events.lock() {
                Ok(mut queue) => queue.push(event),
                Err(poisoned) => poisoned.into_inner().push(event),
            }
            ctx.request_repaint();
        });
    })
}

fn agent() -> Result<ureq::Agent, String> {
    Ok(ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(os_roots()?).build())
        .build()
        .new_agent())
}

fn os_roots() -> Result<ureq::tls::RootCerts, String> {
    let found = rustls_native_certs::load_native_certs();
    let certs: Vec<ureq::tls::Certificate<'static>> = found.certs.iter().map(|c| ureq::tls::Certificate::from_der(c.as_ref()).to_owned()).collect();
    if certs.is_empty() {
        return Err("No trusted certificates found on this system".into());
    }
    Ok(ureq::tls::RootCerts::new_with_certs(&certs))
}

fn lock_value(value: &Mutex<String>) -> String {
    match value.lock() {
        Ok(value) => value.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

fn set_lock(value: &Mutex<String>, new_value: String) {
    match value.lock() {
        Ok(mut value) => *value = new_value,
        Err(poisoned) => *poisoned.into_inner() = new_value,
    }
}

fn request(base: &str, token: &str, action: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    if !(base.starts_with("https://") || base.starts_with("http://localhost") || base.starts_with("http://127.0.0.1")) {
        return Err("Use HTTPS, except for a local test installation.".into());
    }
    let url = format!("{}/app-api.php?action={action}", base.trim_end_matches('/'));
    let agent = agent()?;
    let mut response = if let Some(body) = body {
        let mut request = agent.post(&url).header("Content-Type", "application/json");
        if !token.is_empty() {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
        request.config().http_status_as_error(false).build().send(body.to_string())
    } else {
        let mut request = agent.get(&url);
        if !token.is_empty() {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
        request.config().http_status_as_error(false).build().call()
    }
    .map_err(|error| format!("Web installation not reachable: {error}"))?;
    let status = response.status().as_u16();
    let text = response.body_mut().with_config().limit(310_000_000).read_to_string().map_err(|error| format!("Unreadable Web response: {error}"))?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| format!("Invalid Web response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(value.get("error").and_then(serde_json::Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_string));
    }
    Ok(value)
}

fn request_document(base: &str, token: &str, id: u64) -> Result<Vec<u8>, String> {
    if !(base.starts_with("https://") || base.starts_with("http://localhost") || base.starts_with("http://127.0.0.1")) {
        return Err("Use HTTPS, except for a local test installation.".into());
    }
    let url = format!("{}/app-api.php?action=document-open&id={id}", base.trim_end_matches('/'));
    let mut request = agent()?.get(&url);
    if !token.is_empty() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response =
        request.config().http_status_as_error(false).build().call().map_err(|error| format!("Web installation not reachable: {error}"))?;
    let status = response.status().as_u16();
    let bytes = response.body_mut().with_config().limit(210_000_000).read_to_vec().map_err(|error| format!("Unreadable shared document: {error}"))?;
    if !(200..300).contains(&status) {
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| format!("HTTP {status}"))?;
        return Err(value.get("error").and_then(serde_json::Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_string));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err("The shared file is not a valid PDF.".into());
    }
    Ok(bytes)
}

fn execute(command: Command, base_store: &Mutex<String>, token_store: &Mutex<String>) -> Result<Event, String> {
    if let Command::Connect { url, email, password } = command {
        let value = request(&url, "", "login", Some(serde_json::json!({"email": email, "password": password})))?;
        let token = value.get("token").and_then(serde_json::Value::as_str).ok_or("Access token missing")?.to_string();
        let snapshot =
            serde_json::from_value::<Snapshot>(value.get("snapshot").cloned().ok_or("Snapshot missing")?).map_err(|error| error.to_string())?;
        set_lock(base_store, url);
        set_lock(token_store, token);
        return Ok(Event::Connected(snapshot));
    }
    let base = lock_value(base_store);
    let token = lock_value(token_store);
    if base.is_empty() || token.is_empty() {
        return Err("Connect easyPDF to your Web installation first.".into());
    }
    if matches!(command, Command::Refresh) {
        let value = request(&base, &token, "snapshot", None)?;
        return Ok(Event::Snapshot(
            serde_json::from_value(value.get("snapshot").cloned().ok_or("Snapshot missing")?).map_err(|error| error.to_string())?,
        ));
    }
    if let Command::OpenSharedDocument { id, name } = &command {
        return Ok(Event::SharedDocumentLoaded { name: name.clone(), bytes: request_document(&base, &token, *id)? });
    }
    let task = |task| match task {
        AiTask::Summary => "summary",
        AiTask::Ask => "ask",
        AiTask::Translate => "translate",
    };
    let (action, body) = match command {
        Command::Connect { .. } | Command::Refresh | Command::OpenSharedDocument { .. } => return Err("Invalid connected command.".into()),
        Command::LoadCoverKit { id } => ("cover-load", serde_json::json!({"id": id})),
        Command::SaveCoverKit { name, make_default, front_name, front, back_name, back } => (
            "cover-save",
            serde_json::json!({
                "name": name, "make_default": make_default, "front_name": front_name,
                "front": front.map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes)),
                "back_name": back_name, "back": back.map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes)),
            }),
        ),
        Command::SetDefaultCover { id } => ("cover-default", serde_json::json!({"id": id})),
        Command::DeleteCover { id } => ("cover-delete", serde_json::json!({"id": id})),
        Command::DeleteCovers { ids } => ("cover-delete-many", serde_json::json!({"ids": ids})),
        Command::CreateUser { display_name, email, password, role, organization_id, organization_role } => (
            "user-create",
            serde_json::json!({"display_name": display_name, "email": email, "password": password, "role": role, "organization_id": organization_id, "organization_role": organization_role}),
        ),
        Command::UpdateUser { id, display_name, email, password, role, organization_id, approval_status } => (
            "user-update",
            serde_json::json!({"id": id, "display_name": display_name, "email": email, "password": password, "role": role, "organization_id": organization_id, "approval_status": approval_status}),
        ),
        Command::DeleteUsers { ids } => ("user-delete-many", serde_json::json!({"ids": ids})),
        Command::ToggleUser { id } => ("user-toggle", serde_json::json!({"id": id})),
        Command::SetUserApproval { id, status } => ("user-approval", serde_json::json!({"id": id, "status": status})),
        Command::CreateOrganization { name, slug } => ("organization-create", serde_json::json!({"name": name, "slug": slug})),
        Command::UpdateOrganization { id, name, slug, is_active } => {
            ("organization-update", serde_json::json!({"id": id, "name": name, "slug": slug, "is_active": is_active}))
        }
        Command::DeleteOrganization { id, delete_users } => ("organization-delete", serde_json::json!({"id": id, "delete_users": delete_users})),
        Command::ToggleOrganization { id } => ("organization-toggle", serde_json::json!({"id": id})),
        Command::SaveAi { provider, auth_mode, base_url, model, secret, shared_user_ids } => (
            "ai-save",
            serde_json::json!({"provider": provider, "auth_mode": auth_mode, "base_url": base_url, "model": model, "secret": secret, "shared_user_ids": shared_user_ids}),
        ),
        Command::SetAiSource { source } => ("ai-source", serde_json::json!({"source": source})),
        Command::LoadModels { source, provider, auth_mode, base_url, secret } => {
            ("ai-models", serde_json::json!({"source": source, "provider": provider, "auth_mode": auth_mode, "base_url": base_url, "secret": secret}))
        }
        Command::StartChatGptAuth => ("chatgpt-auth-start", serde_json::json!({})),
        Command::RunAi { task: ai_task, text, question, language } => {
            ("ai-run", serde_json::json!({"task": task(ai_task), "text": text, "question": question, "language": language}))
        }
        Command::ShareDocument { name, bytes, recipient_ids } => (
            "document-share",
            serde_json::json!({
                "name": name,
                "document": base64::engine::general_purpose::STANDARD.encode(bytes),
                "recipient_ids": recipient_ids,
            }),
        ),
        Command::MarkNotificationRead { id } => ("notification-read", serde_json::json!({"id": id})),
        Command::MarkAllNotificationsRead => ("notifications-read-all", serde_json::json!({})),
    };
    let value = request(&base, &token, action, Some(body))?;
    match action {
        "cover-load" => Ok(Event::CoverKitLoaded {
            id: value.get("id").and_then(serde_json::Value::as_u64).ok_or("Cover id missing")?,
            front_name: value.get("front_name").and_then(serde_json::Value::as_str).map(str::to_string),
            front: decode_optional(value.get("front"))?,
            back_name: value.get("back_name").and_then(serde_json::Value::as_str).map(str::to_string),
            back: decode_optional(value.get("back"))?,
        }),
        "ai-models" => {
            Ok(Event::Models(serde_json::from_value(value.get("models").cloned().ok_or("Models missing")?).map_err(|error| error.to_string())?))
        }
        "ai-run" => Ok(Event::AiResult(value.get("result").and_then(serde_json::Value::as_str).ok_or("AI result missing")?.to_string())),
        "chatgpt-auth-start" => {
            Ok(Event::OpenUrl(value.get("url").and_then(serde_json::Value::as_str).ok_or("Authorization URL missing")?.to_string()))
        }
        _ => Ok(Event::Saved {
            message: value.get("message").and_then(serde_json::Value::as_str).unwrap_or("Saved").to_string(),
            snapshot: value.get("snapshot").cloned().map(serde_json::from_value::<Snapshot>).transpose().map_err(|error| error.to_string())?,
        }),
    }
}

fn decode_optional(value: Option<&serde_json::Value>) -> Result<Option<Vec<u8>>, String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|error| error.to_string()))
        .transpose()
}
