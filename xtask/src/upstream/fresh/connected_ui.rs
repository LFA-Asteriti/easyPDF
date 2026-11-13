//! Reconcile the easyPDF connected-state seam with the split v0.17 editor.
//! Exact anchors intentionally fail on a changed upstream layout.

use std::path::Path;

use anyhow::{Result, bail};

use super::{atomic_write, read_bounded, replace_exact_once};

fn after(path: &Path, anchor: &str, addition: &str) -> Result<()> {
    replace_exact_once(path, anchor, &format!("{anchor}{addition}"))
}

pub(super) fn apply(root: &Path, stage: &Path, release: &str, backup_label: &str) -> Result<()> {
    let paths = [
        "crates/ui-egui/src/lib.rs",
        "crates/ui-egui/src/app.rs",
        "crates/ui-egui/src/state.rs",
        "crates/ui-egui/src/dialogs.rs",
        "crates/ui-egui/src/widgets.rs",
        "crates/ui-egui/src/app/prefs.rs",
        "apps/easypdf/Cargo.toml",
        "apps/easypdf/src/main.rs",
        "apps/easypdf/src/portal.rs",
        "crates/ui-egui/src/home.rs",
        "crates/ui-egui/src/workflows/js_ui.rs",
        "crates/ui-egui/src/commands.rs",
        "crates/ui-egui/src/chrome.rs",
        "crates/ui-egui/src/workflows/files.rs",
    ];
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-connected-ui-before-apply"));
    if backup.exists() {
        bail!("connected-UI backup already exists: {}", backup.display());
    }
    for path in paths {
        atomic_write(&backup.join(path), &read_bounded(&stage.join(path))?)?;
    }
    let ui = stage.join("crates/ui-egui/src");
    let lib = ui.join("lib.rs");
    after(&lib, "pub mod i18n;\n", "pub mod connected;\n")?;
    after(
        &lib,
        "    pub generation: u64,\n",
        "    pub selected_kit: Option<u64>,\n    pub save_as_kit: bool,\n    pub make_default: bool,\n    pub kit_name: String,\n",
    )?;
    after(
        &lib,
        "    pub dialog: Option<Dialog>,\n",
        "    /// Preferences and account pages occupy the workspace in both frontends.\n    pub preferences_page: bool,\n    pub connected_page: Option<connected::ConnectedPage>,\n",
    )?;
    after(
        &lib,
        "    pub all_tools_expanded: bool,\n",
        "    pub tool_order: Vec<String>,\n    pub tool_order_changed: Option<Box<dyn FnMut(Vec<String>)>>,\n    pub recommended_tools: Vec<String>,\n    pub recommended_tools_changed: Option<Box<dyn FnMut(Vec<String>)>>,\n    pub menu_tool_order: Vec<String>,\n    pub menu_tool_order_changed: Option<Box<dyn FnMut(Vec<String>)>>,\n    pub portal_url: String,\n    pub portal_email: String,\n    pub connected: connected::State,\n    pub connected_command: Option<connected::CommandFn>,\n    pub connected_events: connected::EventQueue,\n    pub web_logout: Option<Box<dyn FnMut()>>,\n",
    )?;
    let app = ui.join("app.rs");
    after(&app, "            dialog: None,\n", "            preferences_page: false,\n            connected_page: None,\n")?;
    after(
        &app,
        "            all_tools_expanded: false,\n",
        "            tool_order: easypdf_engine::catalog::TOOL_GROUPS.iter().map(|group| group.id.to_string()).collect(),\n            tool_order_changed: None,\n            recommended_tools: [\"organize\", \"comment\", \"form\", \"edit\", \"protect\"].iter().map(|id| (*id).to_owned()).collect(),\n            recommended_tools_changed: None,\n            menu_tool_order: Vec::new(),\n            menu_tool_order_changed: None,\n            portal_url: String::new(),\n            portal_email: String::new(),\n            connected: connected::State {\n                new_user_role: \"user\".into(),\n                new_user_organization_role: \"member\".into(),\n                ai_provider: \"openai\".into(),\n                ai_auth_mode: \"api_key\".into(),\n                ai_language: \"Italiano\".into(),\n                ..Default::default()\n            },\n            connected_command: None,\n            connected_events: Default::default(),\n            web_logout: None,\n",
    )?;
    after(&app, "        self.ctx = Some(ctx.clone());\n", "        self.poll_connected_events();\n        self.poll_connected_background(ctx);\n")?;
    let state = ui.join("state.rs");
    after(&state, "    Covers,\n", "    /// Connected AI actions for the active document.\n    Ai,\n")?;
    let widgets = ui.join("widgets.rs");
    replace_exact_once(
        &widgets,
        "/// A multiline field with the same horizontal inset as [`line()`].",
        r#"/// A masked field with an explicit reveal control; visibility is transient and never saved.
pub fn password_line(ui: &mut egui::Ui, text: &mut dyn egui::TextBuffer, id: egui::Id, hint: &str, width: f32) -> Response {
    let visible_id = id.with("visible");
    let mut visible = ui.ctx().data(|data| data.get_temp::<bool>(visible_id).unwrap_or(false));
    ui.horizontal(|ui| {
        let field_width = if width.is_finite() { width } else { (ui.available_width() - 34.0).max(80.0) };
        let response = ui.add(line(text).id_salt(id).password(!visible).hint_text(hint).desired_width(field_width));
        let (icon, tip) = if visible { ("eye-off", tl!("Hide password")) } else { ("eye", tl!("Show password")) };
        if icons::button(ui, icon, 28.0, false, tip).clicked() {
            visible = !visible;
            ui.ctx().data_mut(|data| data.insert_temp(visible_id, visible));
        }
        response
    })
    .inner
}

/// A multiline field with the same horizontal inset as [`line()`]."#,
    )?;
    let prefs = ui.join("app/prefs.rs");
    after(
        &prefs,
        "            \"language\": self.language,\n",
        "            \"portal_url\": self.portal_url,\n            \"portal_email\": self.portal_email,\n            \"tool_order\": self.tool_order,\n            \"recommended_tools\": self.recommended_tools,\n            \"menu_tool_order\": self.menu_tool_order,\n",
    )?;
    after(
        &prefs,
        "        let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return };\n",
        "        self.portal_url = v.get(\"portal_url\").and_then(serde_json::Value::as_str).unwrap_or_default().chars().take(crate::connected::MAX_PORTAL_URL_CHARS).collect();\n        self.portal_email = v.get(\"portal_email\").and_then(serde_json::Value::as_str).unwrap_or_default().chars().take(crate::connected::MAX_ACCOUNT_FIELD_CHARS).collect();\n        if let Some(order) = v.get(\"tool_order\").and_then(serde_json::Value::as_array) {\n            self.tool_order = order.iter().take(easypdf_engine::catalog::TOOL_GROUPS.len())\n                .filter_map(serde_json::Value::as_str).map(|id| id.chars().take(128).collect()).collect();\n            crate::panels::normalize_tool_order(&mut self.tool_order);\n        }\n        if let Some(recommended) = v.get(\"recommended_tools\").and_then(serde_json::Value::as_array) {\n            self.recommended_tools = recommended.iter().take(256)\n                .filter_map(serde_json::Value::as_str).map(|id| id.chars().take(128).collect()).collect();\n            crate::panels::normalize_recommended_tools(&mut self.recommended_tools);\n        }\n        if let Some(order) = v.get(\"menu_tool_order\").and_then(serde_json::Value::as_array) {\n            self.menu_tool_order = order.iter().take(256)\n                .filter_map(serde_json::Value::as_str).map(|id| id.chars().take(128).collect()).collect();\n            crate::panels::normalize_menu_tool_order(&mut self.menu_tool_order);\n        }\n",
    )?;
    let desktop = stage.join("apps/easypdf");
    after(
        &desktop.join("Cargo.toml"),
        "easypdf-ui-egui = { workspace = true }\n",
        "ureq = { version = \"3.4\", default-features = false, features = [\"rustls-no-provider\", \"_ring\"] }\nrustls-native-certs = \"0.8\"\nbase64 = \"0.22\"\n",
    )?;
    let main = desktop.join("src/main.rs");
    after(&main, "mod motion;\n", "mod portal;\n")?;
    after(
        &main,
        "        app.set_startup_update_check(false);\n",
        "        app.connected_command = Some(portal::command_fn(app.connected_events.clone(), cc.egui_ctx.clone()));\n        app.connected.tab = easypdf_ui_egui::connected::PreferencesTab::Account;\n        app.preferences_page = true;\n",
    )?;
    let portal = desktop.join("src/portal.rs");
    replace_exact_once(&portal, "crate::updates::os_roots()?", "os_roots()?")?;
    replace_exact_once(
        &portal,
        "fn lock_value(value: &Mutex<String>) -> String {",
        "fn os_roots() -> Result<ureq::tls::RootCerts, String> {\n    let found = rustls_native_certs::load_native_certs();\n    let certs: Vec<ureq::tls::Certificate<'static>> = found.certs.iter().map(|c| ureq::tls::Certificate::from_der(c.as_ref()).to_owned()).collect();\n    if certs.is_empty() {\n        return Err(\"No trusted certificates found on this system\".into());\n    }\n    Ok(ureq::tls::RootCerts::new_with_certs(&certs))\n}\n\nfn lock_value(value: &Mutex<String>) -> String {",
    )?;
    wire_pages(&ui, release)?;
    wire_chrome(&ui)?;
    wire_covers(&ui)?;
    Ok(())
}

fn wire_pages(ui: &Path, release: &str) -> Result<()> {
    after(
        &ui.join("home.rs"),
        "    let t = Tokens::get(ui.ctx());\n",
        r#"    if let Some(page) = app.connected_page {
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            app.connected_page = None;
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin { left: 36, right: 36, top: 28, bottom: 28 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                if crate::connected::connected_page_body(ui, app, page, &t) {
                    app.connected_page = None;
                }
            });
        });
        return;
    }
    if app.preferences_page {
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            app.preferences_page = false;
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin { left: 36, right: 36, top: 28, bottom: 28 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                if crate::js_ui::preferences_workspace_body(ui, app, &t) {
                    app.preferences_page = false;
                }
            });
        });
        return;
    }
"#,
    )?;
    let workspace = r#"/// Preferences workspace shared by desktop and web.
pub(crate) fn preferences_workspace_body(ui: &mut egui::Ui, app: &mut EasyPdfApp, t: &Tokens) -> bool {
    @PREFERENCES_HEADER@
    ui.horizontal_wrapped(|ui| {
        use crate::connected::PreferencesTab as P;
        for (tab, label) in [(P::General, "General"), (P::Account, "Account and organization"), (P::Covers, "Cover kits"), (P::Ai, "AI providers")] {
            ui.selectable_value(&mut app.connected.tab, tab, tl!(label));
        }
    });
    ui.separator();
    if app.connected.tab == crate::connected::PreferencesTab::General {
        preferences_body(ui, app, t);
        ui.add_space(12.0);
        preferences_buttons(ui)
    } else {
        crate::connected::preferences_body(ui, app, t)
    }
}

"#;
    let js_ui = ui.join("workflows/js_ui.rs");
    if matches!(release, "v0.18.0" | "v0.18.1" | "v0.18.2" | "v0.19.0") {
        // v0.18 moved preferences into a card-layout module. Keep its search
        // header and body instead of restoring the older v0.17 dialog markup.
        let workspace = workspace.replace("@PREFERENCES_HEADER@", "preferences_header(ui, t);");
        let anchor = "#[expect(unused_imports, reason = \"re-exported for tests and backward compatibility\")]";
        replace_exact_once(&js_ui, anchor, &format!("{workspace}{anchor}"))?;
    } else {
        let workspace = workspace.replace("@PREFERENCES_HEADER@", "ui.label(egui::RichText::new(tl!(\"Preferences\")).font(theme::semibold(18.0)));");
        let anchor = "/// Preferences: interface language and size, appearance, identity and JavaScript. Drawn in a scroll";
        replace_exact_once(&js_ui, anchor, &format!("{workspace}{anchor}"))?;
    }
    if release == "v0.19.0" {
        let anchor = "            ui.add_space(14.0);\n\n            egui::Frame::NONE";
        let card = r#"            ui.add_space(14.0);

            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        widgets::brand_mark(ui, 28.0);
                        ui.label(egui::RichText::new("easyPDF by Massimo Bertolucci").font(theme::semibold(15.0)));
                    });
                    ui.add_space(8.0);
                    let mut open = None;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        for (index, link) in easypdf_engine::links::LINKS.iter().enumerate() {
                            if widgets::icon_pill(ui, link.icon, tl!(link.label), index == 0).on_hover_text(link.url).clicked() {
                                open = Some(link.command);
                            }
                        }
                    });
                    if let Some(command) = open {
                        app.execute(command);
                    }
                });
            ui.add_space(22.0);

            egui::Frame::NONE"#;
        replace_exact_once(&ui.join("home.rs"), anchor, card)?;
    }
    Ok(())
}

fn wire_chrome(ui: &Path) -> Result<()> {
    let commands = ui.join("commands.rs");
    replace_exact_once(
        &commands,
        "            \"app.preferences\" => self.dialog = Some(Dialog::Preferences),",
        r#"            "app.preferences" => {
                self.dialog = None;
                self.active = None;
                self.preferences_page = true;
                self.connected_page = None;
            }
            "ai.summary" => self.open_ai(crate::connected::AiTask::Summary),
            "ai.ask" => self.open_ai(crate::connected::AiTask::Ask),
            "ai.translate" => self.open_ai(crate::connected::AiTask::Translate),"#,
    )?;
    let chrome = ui.join("chrome.rs");
    replace_exact_once(
        &chrome,
        "        app.dialog = Some(Dialog::Preferences);\n        ui.close();",
        "        app.execute(\"app.preferences\");\n        ui.close();",
    )?;
    after(
        &chrome,
        "                        app.execute(\"app.preferences\");\n                    }\n",
        r#"                    if app.connected.snapshot.is_some() {
                        let unread = app.connected.snapshot.as_ref().map_or(0, |snapshot| snapshot.notifications.iter().filter(|item| !item.is_read).count());
                        let tip = if unread == 0 { tl!("Notifications").to_owned() } else { format!("{} ({unread})", tl!("Notifications")) };
                        if icons::button(ui, "bell", button, false, &tip).clicked() {
                            app.active = None;
                            app.preferences_page = false;
                            app.connected_page = Some(crate::connected::ConnectedPage::Notifications);
                        }
                        if icons::button(ui, "folder", button, false, tl!("Shared documents")).clicked() {
                            app.active = None;
                            app.preferences_page = false;
                            app.connected_page = Some(crate::connected::ConnectedPage::SharedDocuments);
                        }
                        if app.active_ids().is_some() && icons::button(ui, "share-2", button, false, tl!("Share document")).clicked() {
                            app.open_share_dialog();
                        }
                    }
                    if app.web_logout.is_some() && icons::button(ui, "external-link", button, false, tl!("Log out")).clicked()
                        && let Some(logout) = app.web_logout.as_mut()
                    { logout(); }
"#,
    )?;
    replace_exact_once(
        &chrome,
        "                    // One click to the community, from anywhere in the app.\n                    if widgets::ghost_button_sized(ui, \"messages-square\", \"Discord\", tab_h).on_hover_text(easypdf_engine::links::DISCORD).clicked() {\n                        app.execute(\"help.discord\");\n                    }\n",
        "",
    )?;
    Ok(())
}

fn wire_covers(ui: &Path) -> Result<()> {
    replace_exact_once(
        &ui.join("commands.rs"),
        "                    self.dialog = Some(Dialog::Covers);\n",
        "                    self.connected.load_default_cover_after_refresh = true;\n                    if self.connected_command.is_some() {\n                        self.connected_send(crate::connected::Command::Refresh);\n                    }\n                    self.dialog = Some(Dialog::Covers);\n",
    )?;
    after(
        &ui.join("workflows/files.rs"),
        "            FilePurpose::CoverFront | FilePurpose::CoverBack => {\n                if let Some((name, bytes)) = files.into_iter().next() {\n",
        "                    self.cover_draft.selected_kit = None;\n",
    )?;
    let dialogs = ui.join("dialogs.rs");
    after(&dialogs, "    dialogs(app, ctx);\n", "    crate::connected::sharing_modal(ctx, app);\n")?;
    after(&dialogs, "            Dialog::Covers => 600.0,\n", "            Dialog::Ai => 700.0,\n")?;
    after(
        &dialogs,
        "        Dialog::Covers => return dialog_covers(ui, app, req),\n",
        "        Dialog::Ai => {\n            req.close = crate::connected::ai_dialog_body(ui, app, &t);\n            return false;\n        }\n",
    )?;
    after(
        &dialogs,
        "    ui.label(tl!(\"Insert every front-cover page before the document and every back-cover page after it.\"));\n",
        r#"    if let Some(snapshot) = app.connected.snapshot.as_ref() && !snapshot.cover_kits.is_empty() {
        let selected = snapshot.cover_kits.iter().find(|kit| Some(kit.id) == app.cover_draft.selected_kit)
            .map_or_else(|| tl!("Choose a cover kit").to_owned(), |kit| kit.name.clone());
        let mut load = None;
        craft_ui::ComboBox::from_id_salt("apply-cover-kit").selected_text(selected).show_ui(ui, |ui| {
            for kit in &snapshot.cover_kits {
                if ui.selectable_label(Some(kit.id) == app.cover_draft.selected_kit, &kit.name).clicked() {
                    load = Some(kit.id);
                    ui.close();
                }
            }
        });
        if let Some(id) = load {
            app.cover_draft.front = None;
            app.cover_draft.back = None;
            app.cover_draft.selected_kit = Some(id);
            app.connected_send(crate::connected::Command::LoadCoverKit { id });
        }
        if app.connected.busy { ui.spinner(); }
    }
"#,
    )?;
    replace_exact_once(
        &dialogs,
        "    ui.label(egui::RichText::new(tl!(\"The covers are applied as one undoable change.\")).color(t.text_muted));",
        r#"    let may_save_kit = app.connected.snapshot.as_ref().is_some_and(|snapshot| snapshot.account.can_manage);
    if may_save_kit && app.cover_draft.selected_kit.is_none() && (app.cover_draft.front.is_some() || app.cover_draft.back.is_some()) {
        ui.add(widgets::checkbox(&mut app.cover_draft.save_as_kit, tl!("Save these covers as a kit")));
        if app.cover_draft.save_as_kit {
            ui.add(crate::widgets::line(&mut app.cover_draft.kit_name).hint_text(tl!("Cover kit name")));
            ui.add(widgets::checkbox(&mut app.cover_draft.make_default, tl!("Use as default cover kit")));
        }
    }
    ui.label(egui::RichText::new(tl!("The covers are applied as one undoable change.")).color(t.text_muted));"#,
    )?;
    replace_exact_once(
        &dialogs,
        "        let ready = app.cover_draft.front.is_some() || app.cover_draft.back.is_some();",
        "        let ready = (app.cover_draft.front.is_some() || app.cover_draft.back.is_some())\n            && (!app.cover_draft.save_as_kit || !app.cover_draft.kit_name.trim().is_empty())\n            && !app.connected.busy;",
    )?;
    replace_exact_once(
        &dialogs,
        "            app.cover_draft = Default::default();\n",
        r#"            let draft = std::mem::take(&mut app.cover_draft);
            if draft.save_as_kit {
                app.connected_send(crate::connected::Command::SaveCoverKit {
                    name: draft.kit_name.trim().to_owned(),
                    make_default: draft.make_default,
                    front_name: draft.front.as_ref().map(|cover| cover.name.clone()),
                    front: draft.front.as_ref().map(|cover| cover.bytes.as_ref().clone()),
                    back_name: draft.back.as_ref().map(|cover| cover.name.clone()),
                    back: draft.back.as_ref().map(|cover| cover.bytes.as_ref().clone()),
                });
            }
"#,
    )?;
    Ok(())
}
