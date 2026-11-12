//! easyPDF in the browser. Build: `cd apps/easypdf-web && trunk build --release`
//! (trunk generates the JS loader; no handwritten JS — plan/execution-plan.md §1).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_arch = "wasm32")]
fn main() {
    use easypdf_ui_egui::EasyPdfApp;
    use eframe::wasm_bindgen::JsCast;

    let options = eframe::WebOptions { renderer: eframe::Renderer::Glow, ..Default::default() };
    wasm_bindgen_futures::spawn_local(async move {
        // The protected portal exposes personal UI preferences. Standalone `trunk serve`
        // simply receives no remote preference and keeps eframe's browser storage.
        let remote_preferences = fetch_tool_preferences().await.ok();
        let remote_connected = fetch_connected_snapshot().await.ok();
        let Some(canvas) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("easypdf"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            return;
        };
        let host_canvas = canvas.clone();
        let error_canvas = canvas.clone();
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(move |cc| {
                    let mut app = EasyPdfApp::new();
                    // "printcraft": settings saved under the app's former name, PrintCraft.
                    if let Some(json) =
                        cc.storage.and_then(|s| s.get_string("easypdf").or_else(|| s.get_string("easypdf")).or_else(|| s.get_string("printcraft")))
                    {
                        app.restore(&json);
                    }
                    if let Some((order, recommended, menu_order, csrf)) = remote_preferences.clone() {
                        let value = serde_json::json!({
                            "tool_order": order,
                            "recommended_tools": recommended,
                            "menu_tool_order": menu_order
                        });
                        app.restore(&value.to_string());
                        let order_csrf = csrf.clone();
                        app.tool_order_changed = Some(Box::new(move |order| {
                            let csrf = order_csrf.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(error) = save_tool_preference(&csrf, "tool_order", &order).await {
                                    eframe::web_sys::console::error_1(&format!("easyPDF: could not synchronize tool order: {error}").into());
                                }
                            });
                        }));
                        let recommended_csrf = csrf.clone();
                        let menu_csrf = csrf.clone();
                        app.recommended_tools_changed = Some(Box::new(move |recommended| {
                            let csrf = recommended_csrf.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(error) = save_tool_preference(&csrf, "recommended_tools", &recommended).await {
                                    eframe::web_sys::console::error_1(&format!("easyPDF: could not synchronize recommended tools: {error}").into());
                                }
                            });
                        }));
                        app.menu_tool_order_changed = Some(Box::new(move |order| {
                            let csrf = menu_csrf.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(error) = save_tool_preference(&csrf, "menu_tool_order", &order).await {
                                    eframe::web_sys::console::error_1(&format!("easyPDF: could not synchronize menu tool order: {error}").into());
                                }
                            });
                        }));
                        let logout_csrf = csrf.clone();
                        app.web_logout = Some(Box::new(move || {
                            let csrf = logout_csrf.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(error) = logout(&csrf).await {
                                    eframe::web_sys::console::error_1(&format!("easyPDF: logout failed: {error}").into());
                                }
                            });
                        }));
                    }
                    if let Some(snapshot) = remote_connected.clone() {
                        let csrf = snapshot.csrf.clone().unwrap_or_default();
                        app.connected.seed_snapshot(snapshot);
                        let events = app.connected_events.clone();
                        let ctx = cc.egui_ctx.clone();
                        app.connected_command = Some(Box::new(move |command| {
                            let csrf = csrf.clone();
                            let events = events.clone();
                            let ctx = ctx.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                let event = send_connected_command(command, &csrf).await.unwrap_or_else(easypdf_ui_egui::connected::Event::Error);
                                match events.lock() {
                                    Ok(mut queue) => queue.push(event),
                                    Err(poisoned) => poisoned.into_inner().push(event),
                                }
                                ctx.request_repaint();
                            });
                        }));
                    }
                    if query_param("preferences").as_deref() == Some("account") {
                        app.preferences_page = true;
                        app.connected.tab = easypdf_ui_egui::connected::PreferencesTab::Account;
                    }
                    // `?file=<url>` opens a PDF from a URL (same-origin or CORS-enabled).
                    if let Some(url) = query_param("file") {
                        let inbox = app.startup_inbox.clone();
                        let failed = app.failed_inbox.clone();
                        let ctx = cc.egui_ctx.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let name = url.rsplit('/').next().unwrap_or("document.pdf").split('?').next().unwrap_or("document.pdf").to_string();
                            match fetch_bytes(&url).await {
                                Ok(bytes) => {
                                    if let Ok(mut q) = inbox.lock() {
                                        q.push((name, bytes));
                                    }
                                }
                                Err(e) => {
                                    eframe::web_sys::console::error_1(&format!("easyPDF: could not fetch {url}: {e}").into());
                                    // Shown in the app too, not only in the console (#173).
                                    if let Ok(mut q) = failed.lock() {
                                        q.push((name, e));
                                    }
                                }
                            }
                            ctx.request_repaint();
                        });
                    }
                    // eframe never raises a close request on a browser reload or tab close, so `guard_quit` never
                    // prompts; ask the browser to confirm instead while a document has unsaved work (#812).
                    warn_before_unload(app.unsaved_flag.clone());
                    // A same-origin iframe or a page hosting the canvas receives files and saves.
                    if let Some(bridge) = host::Bridge::connect(&host_canvas) {
                        // `?author=`: the host's name for the user signs their comments,
                        // unless they chose a name in easyPDF (the web default is "Guest").
                        if let Some(author) = query_param("author").map(|a| a.trim().to_string()).filter(|a| !a.is_empty())
                            && app.comment_prefs.author == "Guest"
                        {
                            app.comment_prefs.author = author.chars().take(200).collect();
                        }
                        bridge.listen(app.startup_inbox.clone(), cc.egui_ctx.clone());
                        app.host_save = Some(bridge.save_fn());
                        app.host_dirty = Some(bridge.dirty_fn());
                        bridge.post_ready();
                    } else if query_param("host").as_deref() == Some("parent") {
                        eframe::web_sys::console::warn_1(
                            &"easyPDF: ?host=parent needs a framing page on the same origin; saves download instead".into(),
                        );
                    }
                    Ok(Box::new(app))
                }),
            )
            .await;
        if let Err(e) = started {
            eframe::web_sys::console::error_1(&e);
            if let Some(bridge) = host::Bridge::connect(&error_canvas) {
                bridge.post_failed(&e.as_string().unwrap_or_else(|| format!("{e:?}")));
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn logout(csrf: &str) -> Result<(), String> {
    use eframe::wasm_bindgen::{JsCast, JsValue};
    let options = web_sys::RequestInit::new();
    options.set_method("POST");
    options.set_mode(web_sys::RequestMode::SameOrigin);
    let encoded = js_sys::encode_uri_component(csrf).as_string().ok_or("could not encode CSRF token")?;
    options.set_body(&JsValue::from_str(&format!("csrf={encoded}")));
    let request = web_sys::Request::new_with_str_and_init("/?action=logout", &options).map_err(|error| format!("{error:?}"))?;
    request.headers().set("Content-Type", "application/x-www-form-urlencoded").map_err(|error| format!("{error:?}"))?;
    let window = web_sys::window().ok_or("no window")?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.map_err(|error| format!("{error:?}"))?;
    let response: web_sys::Response = value.dyn_into().map_err(|_| "not a response")?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    window.location().set_href("/").map_err(|error| format!("{error:?}"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_tool_preferences() -> Result<(Option<Vec<String>>, Option<Vec<String>>, Option<Vec<String>>, String), String> {
    use eframe::wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("no window")?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str("/preferences.php")).await.map_err(|e| format!("{e:?}"))?;
    let response: web_sys::Response = value.dyn_into().map_err(|_| "not a response")?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?
        .as_string()
        .ok_or("preferences response was not text")?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let csrf = value.get("csrf").and_then(serde_json::Value::as_str).ok_or("preferences response has no CSRF token")?.to_string();
    let order = value
        .get("tool_order")
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value::<Vec<String>>(value.clone()).map_err(|e| e.to_string()))
        .transpose()?;
    let recommended = value
        .get("recommended_tools")
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value::<Vec<String>>(value.clone()).map_err(|e| e.to_string()))
        .transpose()?;
    let menu_order = value
        .get("menu_tool_order")
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value::<Vec<String>>(value.clone()).map_err(|e| e.to_string()))
        .transpose()?;
    Ok((order, recommended, menu_order, csrf))
}

#[cfg(target_arch = "wasm32")]
async fn save_tool_preference(csrf: &str, key: &str, values: &[String]) -> Result<(), String> {
    use eframe::wasm_bindgen::{JsCast, JsValue};
    let mut body = serde_json::Map::new();
    body.insert("csrf".into(), serde_json::Value::String(csrf.to_string()));
    body.insert(key.to_string(), serde_json::json!(values));
    let body = serde_json::Value::Object(body).to_string();
    let options = web_sys::RequestInit::new();
    options.set_method("POST");
    options.set_body(&JsValue::from_str(&body));
    options.set_mode(web_sys::RequestMode::SameOrigin);
    let request = web_sys::Request::new_with_str_and_init("/preferences.php", &options).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Content-Type", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or("no window")?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.map_err(|e| format!("{e:?}"))?;
    let response: web_sys::Response = value.dyn_into().map_err(|_| "not a response")?;
    if response.ok() { Ok(()) } else { Err(format!("HTTP {}", response.status())) }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_connected_snapshot() -> Result<easypdf_ui_egui::connected::Snapshot, String> {
    let value = app_api("snapshot", "GET", None).await?;
    serde_json::from_value(value.get("snapshot").cloned().ok_or("snapshot missing")?).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn app_api(action: &str, method: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    use eframe::wasm_bindgen::{JsCast, JsValue};
    let options = web_sys::RequestInit::new();
    options.set_method(method);
    options.set_mode(web_sys::RequestMode::SameOrigin);
    if let Some(body) = body {
        options.set_body(&JsValue::from_str(&body.to_string()));
    }
    let request =
        web_sys::Request::new_with_str_and_init(&format!("/app-api.php?action={action}"), &options).map_err(|error| format!("{error:?}"))?;
    request.headers().set("Content-Type", "application/json").map_err(|error| format!("{error:?}"))?;
    let window = web_sys::window().ok_or("no window")?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.map_err(|error| format!("{error:?}"))?;
    let response: web_sys::Response = value.dyn_into().map_err(|_| "not a response")?;
    let status = response.status();
    let text = wasm_bindgen_futures::JsFuture::from(response.text().map_err(|error| format!("{error:?}"))?)
        .await
        .map_err(|error| format!("{error:?}"))?
        .as_string()
        .ok_or("response was not text")?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(value.get("error").and_then(serde_json::Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_string));
    }
    Ok(value)
}

#[cfg(target_arch = "wasm32")]
async fn app_api_document(id: u64) -> Result<Vec<u8>, String> {
    use eframe::wasm_bindgen::JsCast;
    let options = web_sys::RequestInit::new();
    options.set_method("GET");
    options.set_mode(web_sys::RequestMode::SameOrigin);
    let request = web_sys::Request::new_with_str_and_init(&format!("/app-api.php?action=document-open&id={id}"), &options)
        .map_err(|error| format!("{error:?}"))?;
    let window = web_sys::window().ok_or("no window")?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.map_err(|error| format!("{error:?}"))?;
    let response: web_sys::Response = value.dyn_into().map_err(|_| "not a response")?;
    let status = response.status();
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().map_err(|error| format!("{error:?}"))?)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    if !response.ok() {
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| format!("HTTP {status}"))?;
        return Err(value.get("error").and_then(serde_json::Value::as_str).map_or_else(|| format!("HTTP {status}"), str::to_string));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err("The shared file is not a valid PDF.".into());
    }
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
async fn send_connected_command(command: easypdf_ui_egui::connected::Command, csrf: &str) -> Result<easypdf_ui_egui::connected::Event, String> {
    use base64::Engine as _;
    use easypdf_ui_egui::connected::{AiTask, Command, Event, Snapshot};
    let task = |task| match task {
        AiTask::Summary => "summary",
        AiTask::Ask => "ask",
        AiTask::Translate => "translate",
    };
    if let Command::OpenSharedDocument { id, name } = &command {
        return Ok(Event::SharedDocumentLoaded { name: name.clone(), bytes: app_api_document(*id).await? });
    }
    let (action, mut body) = match command {
        Command::Connect { .. } => return Err("The Web app already uses its signed-in account.".into()),
        Command::Refresh => {
            let snapshot = fetch_connected_snapshot().await?;
            return Ok(Event::Snapshot(snapshot));
        }
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
        Command::OpenSharedDocument { .. } => return Err("Invalid connected command.".into()),
        Command::MarkNotificationRead { id } => ("notification-read", serde_json::json!({"id": id})),
        Command::MarkAllNotificationsRead => ("notifications-read-all", serde_json::json!({})),
    };
    body["csrf"] = serde_json::Value::String(csrf.to_string());
    let value = app_api(action, "POST", Some(body)).await?;
    match action {
        "cover-load" => Ok(Event::CoverKitLoaded {
            id: value.get("id").and_then(serde_json::Value::as_u64).ok_or("cover id missing")?,
            front_name: value.get("front_name").and_then(serde_json::Value::as_str).map(str::to_string),
            front: value
                .get("front")
                .and_then(serde_json::Value::as_str)
                .map(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|error| error.to_string()))
                .transpose()?,
            back_name: value.get("back_name").and_then(serde_json::Value::as_str).map(str::to_string),
            back: value
                .get("back")
                .and_then(serde_json::Value::as_str)
                .map(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|error| error.to_string()))
                .transpose()?,
        }),
        "ai-models" => {
            Ok(Event::Models(serde_json::from_value(value.get("models").cloned().ok_or("models missing")?).map_err(|error| error.to_string())?))
        }
        "ai-run" => Ok(Event::AiResult(value.get("result").and_then(serde_json::Value::as_str).ok_or("AI result missing")?.to_string())),
        "chatgpt-auth-start" => {
            Ok(Event::OpenUrl(value.get("url").and_then(serde_json::Value::as_str).ok_or("authorization URL missing")?.to_string()))
        }
        _ => Ok(Event::Saved {
            message: value.get("message").and_then(serde_json::Value::as_str).unwrap_or("Saved").to_string(),
            snapshot: value.get("snapshot").cloned().map(serde_json::from_value::<Snapshot>).transpose().map_err(|error| error.to_string())?,
        }),
    }
}

#[cfg(target_arch = "wasm32")]
fn warn_before_unload(unsaved: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use eframe::wasm_bindgen::{JsCast, closure::Closure};
    let Some(window) = web_sys::window() else { return };
    let handler = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        if unsaved.load(std::sync::atomic::Ordering::Relaxed) {
            event.prevent_default();
        }
    });
    if window.add_event_listener_with_callback("beforeunload", handler.as_ref().unchecked_ref()).is_ok() {
        handler.forget();
    }
}

#[cfg(target_arch = "wasm32")]
fn query_param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search).ok()?.get(key)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    use eframe::wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("no window")?;
    let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await.map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "not a response")?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let buf = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?).await.map_err(|e| format!("{e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// The host bridge uses postMessage for a same-origin parent iframe or CustomEvents on a
/// `data-host="page"` canvas. Parent mode wins when both are requested.
///
/// - `easypdf:ready` (to the parent): the app is listening; send files now.
/// - `easypdf:open` (from the parent): `{ name, bytes }`, where `bytes` is an `ArrayBuffer` or
///   `Uint8Array` holding a PDF. It opens in a tab, like a `?file=` URL.
/// - `easypdf:save` (to the parent): `{ name, saveAs, bytes }` when the user saves; `bytes` is
///   a transferred `ArrayBuffer`. The parent stores the file (and reports its own errors).
/// - `easypdf:dirty` (to the parent): `{ dirty }` whenever unsaved work appears or goes away.
/// - `easypdf:failed` (to the parent): `{ error }` when the app couldn't start.
///
/// Parent messages are restricted to the same origin and the actual framing window.
#[cfg(target_arch = "wasm32")]
mod host {
    use eframe::wasm_bindgen::{JsCast, JsValue, closure::Closure};

    #[derive(Clone)]
    pub struct Bridge {
        transport: Transport,
    }

    #[derive(Clone)]
    enum Transport {
        Parent { window: web_sys::Window, parent: web_sys::Window, origin: String },
        Page { canvas: web_sys::EventTarget },
    }

    impl Bridge {
        /// A cross-origin iframe has no bridge, so saves fall back to downloads.
        pub fn connect(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
            let window = web_sys::window()?;
            if super::query_param("host").as_deref() == Some("parent") {
                let parent = window.parent().ok().flatten()?;
                if js_sys::Object::is(&parent, &window) {
                    return None;
                }
                let origin = window.location().origin().ok()?;
                if parent.location().origin().ok()? != origin {
                    return None;
                }
                Some(Self { transport: Transport::Parent { window, parent, origin } })
            } else if canvas.get_attribute("data-host").as_deref() == Some("page") {
                Some(Self { transport: Transport::Page { canvas: canvas.clone().into() } })
            } else {
                None
            }
        }

        fn post(&self, message: &js_sys::Object, transfer: Option<&JsValue>) -> Result<(), String> {
            match &self.transport {
                Transport::Parent { parent, origin, .. } => {
                    let sent = match transfer {
                        Some(t) => parent.post_message_with_transfer(message, origin, &js_sys::Array::of1(t)),
                        None => parent.post_message(message, origin),
                    };
                    sent.map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
                }
                Transport::Page { canvas } => {
                    let name = string(&message.clone().into(), "type").ok_or("host message has no type")?;
                    let init = web_sys::CustomEventInit::new();
                    init.set_detail(&message.clone().into());
                    let event = web_sys::CustomEvent::new_with_event_init_dict(&name, &init)
                        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
                    canvas.dispatch_event(&event).map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
                    Ok(())
                }
            }
        }

        pub fn post_ready(&self) {
            if let Some(m) = message(&[
                ("type", "easypdf:ready".into()),
                ("version", "1".into()),
                ("appVersion", env!("CARGO_PKG_VERSION").into()),
                ("file", false.into()),
            ]) {
                let _ = self.post(&m, None);
            }
        }

        /// The app couldn't start (no WebGL, for example): let the parent say so.
        pub fn post_failed(&self, error: &str) {
            if let Some(m) = message(&[("type", "easypdf:failed".into()), ("error", error.into())]) {
                let _ = self.post(&m, None);
            }
        }

        /// Hear `easypdf:open` from the parent and queue the file like a `?file=` download.
        pub fn listen(&self, inbox: easypdf_ui_egui::Inbox, ctx: egui::Context) {
            match &self.transport {
                Transport::Parent { window, parent, origin } => {
                    let (parent, origin, window) = (parent.clone(), origin.clone(), window.clone());
                    let on_message = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
                        if event.origin() == origin && event.source().is_some_and(|s| js_sys::Object::is(&s, &parent)) {
                            queue_open(&event.data(), &inbox, &ctx);
                        }
                    });
                    if window.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref()).is_ok() {
                        on_message.forget();
                    }
                }
                Transport::Page { canvas } => {
                    let on_event = Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |event: web_sys::CustomEvent| {
                        queue_open(&event.detail(), &inbox, &ctx);
                    });
                    if canvas.add_event_listener_with_callback("easypdf:open", on_event.as_ref().unchecked_ref()).is_ok() {
                        on_event.forget();
                    }
                }
            }
        }

        pub fn save_fn(&self) -> easypdf_ui_egui::HostSaveFn {
            let bridge = self.clone();
            Box::new(move |name: &str, bytes: &[u8], save_as: bool| {
                let buffer = js_sys::Uint8Array::from(bytes).buffer();
                let m =
                    message(&[("type", "easypdf:save".into()), ("name", name.into()), ("saveAs", save_as.into()), ("bytes", buffer.clone().into())])
                        .ok_or_else(|| "couldn't build the message".to_string())?;
                bridge.post(&m, Some(&buffer))
            })
        }

        pub fn dirty_fn(&self) -> easypdf_ui_egui::HostDirtyFn {
            let bridge = self.clone();
            let mut last = false;
            Box::new(move |dirty: bool| {
                if dirty != last {
                    last = dirty;
                    if let Some(m) = message(&[("type", "easypdf:dirty".into()), ("dirty", dirty.into())]) {
                        let _ = bridge.post(&m, None);
                    }
                }
            })
        }
    }

    fn message(fields: &[(&str, JsValue)]) -> Option<js_sys::Object> {
        let m = js_sys::Object::new();
        for (key, value) in fields {
            js_sys::Reflect::set(&m, &JsValue::from_str(key), value).ok()?;
        }
        Some(m)
    }

    fn string(data: &JsValue, key: &str) -> Option<String> {
        js_sys::Reflect::get(data, &JsValue::from_str(key)).ok()?.as_string()
    }

    fn queue_open(data: &JsValue, inbox: &easypdf_ui_egui::Inbox, ctx: &egui::Context) {
        if string(data, "type").as_deref().is_some_and(|kind| kind != "easypdf:open") {
            return;
        }
        let name = string(data, "name").unwrap_or_default();
        let name = name.rsplit(['/', '\\']).next().unwrap_or_default();
        let name: String = name.chars().filter(|c| !c.is_control()).take(255).collect();
        let name = if name.trim().is_empty() || name == "." || name == ".." { "document.pdf".to_string() } else { name };
        let Ok(bytes) = js_sys::Reflect::get(data, &JsValue::from_str("bytes")) else { return };
        let array = if let Some(array) = bytes.dyn_ref::<js_sys::Uint8Array>() {
            array.clone()
        } else if bytes.is_instance_of::<js_sys::ArrayBuffer>() {
            js_sys::Uint8Array::new(&bytes)
        } else {
            return;
        };
        if array.length() as u64 > 512 << 20 {
            eframe::web_sys::console::warn_1(&"easyPDF: host file exceeds the 512 MiB open limit".into());
            return;
        }
        if let Ok(mut q) = inbox.lock() {
            q.push((name, array.to_vec()));
        }
        ctx.request_repaint();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("easypdf-web targets wasm32: run `trunk serve` or `trunk build --release` in apps/easypdf-web");
}
