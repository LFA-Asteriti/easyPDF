//! Account, organisation, shared cover-kit and AI-provider state used by both frontends.
//!
//! The UI owns no network client. A native or Web host installs [`CommandFn`] and feeds
//! [`Event`] values back through [`EventQueue`].

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::EasyPdfApp;

pub const MAX_PORTAL_URL_CHARS: usize = 2_048;
pub const MAX_ACCOUNT_FIELD_CHARS: usize = 254;
pub const MAX_AI_TEXT_CHARS: usize = 500_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreferencesTab {
    #[default]
    General,
    Account,
    Covers,
    Ai,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectedPage {
    Notifications,
    SharedDocuments,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub id: u64,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub organization_id: Option<u64>,
    pub organization_name: Option<String>,
    pub organization_role: Option<String>,
    pub can_manage: bool,
    #[serde(default)]
    pub can_manage_accounts: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserInfo {
    pub id: u64,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub organization_id: Option<u64>,
    pub organization_name: Option<String>,
    pub organization_role: Option<String>,
    pub is_active: bool,
    #[serde(default = "approved_status")]
    pub approval_status: String,
}

fn approved_status() -> String {
    "approved".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrganizationInfo {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub is_active: bool,
    pub member_count: u64,
}

#[derive(Clone, Debug, Default)]
pub struct UserEditor {
    pub id: u64,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub organization_id: Option<u64>,
    pub approval_status: String,
    pub password: String,
    pub password_confirm: String,
}

#[derive(Clone, Debug, Default)]
pub struct OrganizationEditor {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverKitInfo {
    pub id: u64,
    pub name: String,
    pub front_name: Option<String>,
    pub back_name: Option<String>,
    pub is_default: bool,
    #[serde(default)]
    pub front_preview: Option<String>,
    #[serde(default)]
    pub back_preview: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiSettings {
    pub provider: String,
    pub auth_mode: String,
    pub base_url: String,
    pub model: String,
    pub has_secret: bool,
    #[serde(default)]
    pub owner_user_id: Option<u64>,
    #[serde(default)]
    pub owner_name: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShareUser {
    pub id: u64,
    pub display_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationInfo {
    pub id: u64,
    pub document_id: Option<u64>,
    pub actor_name: String,
    pub document_name: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedDocumentInfo {
    pub id: u64,
    pub name: String,
    pub byte_size: u64,
    pub sender_name: String,
    pub is_opened: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub account: Account,
    pub users: Vec<UserInfo>,
    pub organizations: Vec<OrganizationInfo>,
    pub cover_kits: Vec<CoverKitInfo>,
    pub ai: AiSettings,
    #[serde(default)]
    pub personal_ai: AiSettings,
    #[serde(default)]
    pub shared_ai: Option<AiSettings>,
    #[serde(default = "personal_ai_source")]
    pub ai_source: String,
    #[serde(default)]
    pub ai_shared_user_ids: Vec<u64>,
    #[serde(default)]
    pub share_users: Vec<ShareUser>,
    #[serde(default)]
    pub notifications: Vec<NotificationInfo>,
    #[serde(default)]
    pub shared_documents: Vec<SharedDocumentInfo>,
    pub csrf: Option<String>,
}

fn personal_ai_source() -> String {
    "personal".into()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiTask {
    #[default]
    Summary,
    Ask,
    Translate,
}

#[derive(Clone, Debug)]
pub enum Command {
    Connect {
        url: String,
        email: String,
        password: String,
    },
    Refresh,
    LoadCoverKit {
        id: u64,
    },
    SaveCoverKit {
        name: String,
        make_default: bool,
        front_name: Option<String>,
        front: Option<Vec<u8>>,
        back_name: Option<String>,
        back: Option<Vec<u8>>,
    },
    SetDefaultCover {
        id: u64,
    },
    DeleteCover {
        id: u64,
    },
    DeleteCovers {
        ids: Vec<u64>,
    },
    CreateUser {
        display_name: String,
        email: String,
        password: String,
        role: String,
        organization_id: Option<u64>,
        organization_role: Option<String>,
    },
    UpdateUser {
        id: u64,
        display_name: String,
        email: String,
        password: String,
        role: String,
        organization_id: Option<u64>,
        approval_status: String,
    },
    DeleteUsers {
        ids: Vec<u64>,
    },
    ToggleUser {
        id: u64,
    },
    SetUserApproval {
        id: u64,
        status: String,
    },
    CreateOrganization {
        name: String,
        slug: String,
    },
    UpdateOrganization {
        id: u64,
        name: String,
        slug: String,
        is_active: bool,
    },
    DeleteOrganization {
        id: u64,
        delete_users: bool,
    },
    ToggleOrganization {
        id: u64,
    },
    SaveAi {
        provider: String,
        auth_mode: String,
        base_url: String,
        model: String,
        secret: String,
        shared_user_ids: Vec<u64>,
    },
    SetAiSource {
        source: String,
    },
    LoadModels {
        source: String,
        provider: String,
        auth_mode: String,
        base_url: String,
        secret: String,
    },
    StartChatGptAuth,
    RunAi {
        task: AiTask,
        text: String,
        question: String,
        language: String,
    },
    ShareDocument {
        name: String,
        bytes: Vec<u8>,
        recipient_ids: Vec<u64>,
    },
    OpenSharedDocument {
        id: u64,
        name: String,
    },
    MarkNotificationRead {
        id: u64,
    },
    MarkAllNotificationsRead,
}

#[derive(Clone, Debug)]
pub enum Event {
    Connected(Snapshot),
    Snapshot(Snapshot),
    CoverKitLoaded { id: u64, front_name: Option<String>, front: Option<Vec<u8>>, back_name: Option<String>, back: Option<Vec<u8>> },
    Models(Vec<String>),
    AiResult(String),
    SharedDocumentLoaded { name: String, bytes: Vec<u8> },
    OpenUrl(String),
    Saved { message: String, snapshot: Option<Snapshot> },
    Error(String),
}

pub type EventQueue = Arc<Mutex<Vec<Event>>>;
pub type CommandFn = Box<dyn FnMut(Command)>;

#[derive(Default)]
pub struct State {
    pub tab: PreferencesTab,
    pub snapshot: Option<Snapshot>,
    pub busy: bool,
    pub error: Option<String>,
    pub password: String,
    pub selected_cover: Option<u64>,
    pub selected_cover_ids: BTreeSet<u64>,
    pub selected_user_ids: BTreeSet<u64>,
    pub selected_organization_ids: BTreeSet<u64>,
    /// Apply covers asked for a fresh snapshot and should load its default kit afterwards.
    pub load_default_cover_after_refresh: bool,
    pub new_kit_name: String,
    pub new_user_name: String,
    pub new_user_email: String,
    pub new_user_password: String,
    pub new_user_password_confirm: String,
    pub new_user_role: String,
    pub new_user_organization: Option<u64>,
    pub new_user_organization_role: String,
    pub new_organization_name: String,
    pub new_organization_slug: String,
    pub user_editor: Option<UserEditor>,
    pub organization_editor: Option<OrganizationEditor>,
    pub confirm_delete_users: bool,
    pub delete_organization_id: Option<u64>,
    pub delete_organization_users: bool,
    pub ai_provider: String,
    pub ai_source: String,
    pub ai_auth_mode: String,
    pub ai_base_url: String,
    pub ai_model: String,
    pub ai_secret: String,
    pub ai_models: Vec<String>,
    pub ai_shared_user_ids: BTreeSet<u64>,
    pub ai_question: String,
    pub ai_language: String,
    pub ai_result: String,
    pub ai_task: AiTask,
    pub share_dialog: bool,
    pub share_recipient_ids: BTreeSet<u64>,
    pub shared_documents_grid: bool,
    pub next_background_refresh: f64,
}

impl State {
    pub fn seed_snapshot(&mut self, snapshot: Snapshot) {
        self.selected_cover = snapshot.cover_kits.iter().find(|kit| kit.is_default).or_else(|| snapshot.cover_kits.first()).map(|kit| kit.id);
        self.selected_user_ids.retain(|id| snapshot.users.iter().any(|user| user.id == *id));
        self.selected_organization_ids.retain(|id| snapshot.organizations.iter().any(|organization| organization.id == *id));
        if snapshot.account.role != "super_admin" {
            self.new_user_organization = snapshot.account.organization_id;
            self.new_user_role = "user".into();
        } else if !matches!(self.new_user_role.as_str(), "super_admin" | "admin" | "user") {
            self.new_user_role = "user".into();
        }
        self.ai_provider = snapshot.ai.provider.clone();
        self.ai_source = snapshot.ai_source.clone();
        self.ai_auth_mode = snapshot.ai.auth_mode.clone();
        self.ai_base_url = snapshot.ai.base_url.clone();
        self.ai_model = snapshot.ai.model.clone();
        self.ai_shared_user_ids = snapshot.ai_shared_user_ids.iter().copied().collect();
        self.snapshot = Some(snapshot);
        self.busy = false;
        self.error = None;
    }
}

impl EasyPdfApp {
    pub(crate) fn open_share_dialog(&mut self) {
        let Some(snapshot) = self.connected.snapshot.as_ref() else {
            self.notify_tr("Connect easyPDF to your Web installation first.");
            return;
        };
        if snapshot.account.organization_id.is_none() {
            self.notify_tr("Document sharing requires an organization account.");
            return;
        }
        if self.active_ids().is_none() {
            self.notify_tr("Open a PDF first");
            return;
        }
        self.connected.share_recipient_ids.clear();
        self.connected.share_dialog = true;
    }

    fn share_active_document(&mut self) {
        let Some((_, id)) = self.active_ids() else {
            self.notify_tr("Open a PDF first");
            return;
        };
        let Some(name) = self.session.get(id).map(|document| document.name.clone()) else {
            self.notify_tr("Open a PDF first");
            return;
        };
        let bytes = match self.session.save_bytes(id) {
            Ok(bytes) => bytes.as_ref().clone(),
            Err(error) => {
                self.notify(error.to_string());
                return;
            }
        };
        let recipient_ids = self.connected.share_recipient_ids.iter().copied().collect();
        self.connected_send(Command::ShareDocument { name, bytes, recipient_ids });
        self.connected.share_dialog = false;
        self.connected.share_recipient_ids.clear();
    }

    pub(crate) fn open_ai(&mut self, task: AiTask) {
        self.connected.ai_task = task;
        self.connected.ai_result.clear();
        self.dialog = Some(crate::Dialog::Ai);
    }

    pub(crate) fn run_ai(&mut self) {
        let Some((_, id)) = self.active_ids() else {
            self.notify_tr("Open a PDF first");
            return;
        };
        let Some(document) = self.session.get(id) else { return };
        let mut exporter = easypdf_engine::export::Exporter::new(document);
        let pages: Vec<usize> = (0..document.info.pages.len()).collect();
        let extracted = if self.connected.ai_task == AiTask::Ask {
            let mut marked = String::new();
            for page in &pages {
                match exporter.text_of(&[*page]) {
                    Ok(page_text) => {
                        marked.push_str(&format!("[Page {}]\n{}\n\n", page + 1, page_text));
                    }
                    Err(error) => {
                        self.notify(error);
                        return;
                    }
                }
            }
            Ok(marked)
        } else {
            exporter.text_of(&pages)
        };
        let text = match extracted {
            Ok(text) if text.chars().count() <= MAX_AI_TEXT_CHARS => text,
            Ok(_) => {
                self.notify_tr("Document text is too large for AI. Use a smaller document.");
                return;
            }
            Err(error) => {
                self.notify(error);
                return;
            }
        };
        self.connected_send(Command::RunAi {
            task: self.connected.ai_task,
            text,
            question: self.connected.ai_question.trim().to_string(),
            language: self.connected.ai_language.trim().to_string(),
        });
    }

    pub fn connected_send(&mut self, command: Command) {
        let Some(send) = self.connected_command.as_mut() else {
            self.connected.error = Some(tl!("Connect easyPDF to your Web installation first.").to_string());
            return;
        };
        self.connected.busy = true;
        self.connected.error = None;
        send(command);
    }

    pub(crate) fn poll_connected_events(&mut self) {
        let events = match self.connected_events.lock() {
            Ok(mut queue) => std::mem::take(&mut *queue),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };
        for event in events {
            match event {
                Event::Connected(snapshot) => self.connected.seed_snapshot(snapshot),
                Event::Snapshot(snapshot) => {
                    self.connected.seed_snapshot(snapshot);
                    if std::mem::take(&mut self.connected.load_default_cover_after_refresh)
                        && let Some(id) = self.connected.selected_cover
                    {
                        self.cover_draft.selected_kit = Some(id);
                        self.connected_send(Command::LoadCoverKit { id });
                    }
                }
                Event::CoverKitLoaded { id, front_name, front, back_name, back } => {
                    self.connected.busy = false;
                    self.connected.selected_cover = Some(id);
                    self.cover_draft.front = front.zip(front_name).map(|(bytes, name)| easypdf_engine::CoverFile { name, bytes: Arc::new(bytes) });
                    self.cover_draft.back = back.zip(back_name).map(|(bytes, name)| easypdf_engine::CoverFile { name, bytes: Arc::new(bytes) });
                    self.cover_draft.selected_kit = Some(id);
                }
                Event::Models(models) => {
                    self.connected.busy = false;
                    self.connected.ai_models = models;
                }
                Event::AiResult(result) => {
                    self.connected.busy = false;
                    self.connected.ai_result = result;
                }
                Event::SharedDocumentLoaded { name, bytes } => {
                    self.connected.busy = false;
                    match self.inbox.lock() {
                        Ok(mut inbox) => inbox.push((name, bytes)),
                        Err(poisoned) => poisoned.into_inner().push((name, bytes)),
                    }
                }
                Event::OpenUrl(url) => {
                    self.connected.busy = false;
                    self.open_url(&url);
                }
                Event::Saved { message, snapshot } => {
                    self.connected.busy = false;
                    if let Some(snapshot) = snapshot {
                        self.connected.seed_snapshot(snapshot);
                    }
                    self.notify(message);
                }
                Event::Error(error) => {
                    self.connected.busy = false;
                    self.connected.load_default_cover_after_refresh = false;
                    self.connected.error = Some(error.clone());
                    self.notify(error);
                }
            }
        }
    }

    pub(crate) fn poll_connected_background(&mut self, ctx: &egui::Context) {
        if self.connected.snapshot.as_ref().and_then(|snapshot| snapshot.account.organization_id).is_none() {
            return;
        }
        let now = ctx.input(|input| input.time);
        if !self.connected.busy && now >= self.connected.next_background_refresh {
            self.connected.next_background_refresh = now + 30.0;
            self.connected_send(Command::Refresh);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(30));
    }
}

pub(crate) fn sharing_modal(ctx: &egui::Context, app: &mut EasyPdfApp) {
    if !app.connected.share_dialog {
        return;
    }
    let Some(snapshot) = app.connected.snapshot.clone() else {
        app.connected.share_dialog = false;
        return;
    };
    let mut close = false;
    let mut share = false;
    let modal = crate::widgets::show_modal(ctx, "share-document", |ui| {
        ui.set_width(520.0);
        ui.label(egui::RichText::new(tl!("Share document")).font(crate::theme::semibold(18.0)));
        ui.label(
            egui::RichText::new(tl!("A protected snapshot including current unsaved changes will be shared."))
                .small()
                .color(crate::theme::Tokens::get(ui.ctx()).text_muted),
        );
        ui.add_space(10.0);
        if snapshot.share_users.is_empty() {
            ui.label(tl!("No other active users are available in this organization."));
        } else {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for member in &snapshot.share_users {
                    let mut selected = app.connected.share_recipient_ids.contains(&member.id);
                    if ui.add(crate::widgets::checkbox(&mut selected, &member.display_name)).changed() {
                        if selected {
                            app.connected.share_recipient_ids.insert(member.id);
                        } else {
                            app.connected.share_recipient_ids.remove(&member.id);
                        }
                    }
                }
            });
        }
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled_ui(!app.connected.busy && !app.connected.share_recipient_ids.is_empty(), |ui| {
                    crate::widgets::pill_button(ui, tl!("Share"), true)
                })
                .inner
                .clicked()
            {
                share = true;
            }
            if crate::widgets::pill_button(ui, tl!("Cancel"), false).clicked() {
                close = true;
            }
        });
    });
    if share {
        app.share_active_document();
    } else if close || modal.should_close() {
        app.connected.share_dialog = false;
        app.connected.share_recipient_ids.clear();
    }
}

pub(crate) fn connected_page_body(ui: &mut egui::Ui, app: &mut EasyPdfApp, page: ConnectedPage, t: &crate::theme::Tokens) -> bool {
    let Some(snapshot) = app.connected.snapshot.clone() else {
        ui.label(tl!("Connect the desktop app to your easyPDF Web installation."));
        return crate::widgets::pill_button(ui, tl!("Close"), false).clicked();
    };
    let mut action = None;
    let mut close = false;
    ui.horizontal(|ui| {
        let title = match page {
            ConnectedPage::Notifications => tl!("Notifications"),
            ConnectedPage::SharedDocuments => tl!("Shared documents"),
        };
        ui.label(egui::RichText::new(title).font(crate::theme::semibold(24.0)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::widgets::pill_button(ui, tl!("Close"), false).clicked() {
                close = true;
            }
            if ui.add_enabled_ui(!app.connected.busy, |ui| crate::widgets::pill_button(ui, tl!("Refresh"), false)).inner.clicked() {
                action = Some(Command::Refresh);
            }
        });
    });
    ui.separator();
    match page {
        ConnectedPage::Notifications => {
            let unread = snapshot.notifications.iter().filter(|item| !item.is_read).count();
            if unread > 0
                && ui.add_enabled_ui(!app.connected.busy, |ui| crate::widgets::pill_button(ui, tl!("Mark all as read"), false)).inner.clicked()
            {
                action = Some(Command::MarkAllNotificationsRead);
            }
            if snapshot.notifications.is_empty() {
                ui.label(egui::RichText::new(tl!("No notifications.")).color(t.text_muted));
            } else {
                egui::Grid::new("notification-table").striped(true).min_col_width(100.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(tl!("Status")).strong());
                    ui.label(egui::RichText::new(tl!("Notification")).strong());
                    ui.label(egui::RichText::new(tl!("Date")).strong());
                    ui.label("");
                    ui.end_row();
                    for item in &snapshot.notifications {
                        ui.label(if item.is_read { tl_ctx!("notification status", "Read") } else { tl!("New") });
                        ui.label(format!("{} {} {}", item.actor_name, tl!("shared"), item.document_name));
                        ui.label(&item.created_at);
                        if !item.is_read && ui.small_button(tl!("Mark as read")).clicked() {
                            action = Some(Command::MarkNotificationRead { id: item.id });
                        }
                        ui.end_row();
                    }
                });
            }
        }
        ConnectedPage::SharedDocuments => {
            ui.horizontal(|ui| {
                ui.label(tl_ctx!("shared documents", "View"));
                crate::widgets::radio_value(ui, &mut app.connected.shared_documents_grid, false, tl!("List"));
                crate::widgets::radio_value(ui, &mut app.connected.shared_documents_grid, true, tl!("Grid"));
            });
            if snapshot.shared_documents.is_empty() {
                ui.label(egui::RichText::new(tl!("No documents have been shared with you.")).color(t.text_muted));
            } else if app.connected.shared_documents_grid {
                ui.horizontal_wrapped(|ui| {
                    for document in &snapshot.shared_documents {
                        egui::Frame::NONE
                            .fill(t.card)
                            .stroke(egui::Stroke::new(1.0, t.border))
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(210.0);
                                ui.add(crate::icons::image("file-text", 24.0, t.accent));
                                ui.label(egui::RichText::new(&document.name).strong());
                                ui.label(
                                    egui::RichText::new(format!("{} · {}", document.sender_name, document.created_at)).small().color(t.text_muted),
                                );
                                if ui.small_button(tl!("Open")).clicked() {
                                    action = Some(Command::OpenSharedDocument { id: document.id, name: document.name.clone() });
                                }
                            });
                    }
                });
            } else {
                egui::Grid::new("shared-document-table").striped(true).min_col_width(100.0).show(ui, |ui| {
                    for heading in [tl!("Name"), tl!("Shared by"), tl!("Size"), tl!("Date")] {
                        ui.label(egui::RichText::new(heading).strong());
                    }
                    ui.label("");
                    ui.end_row();
                    for document in &snapshot.shared_documents {
                        ui.label(&document.name);
                        ui.label(&document.sender_name);
                        ui.label(crate::panels::human_size(usize::try_from(document.byte_size).unwrap_or(usize::MAX)));
                        ui.label(&document.created_at);
                        if ui.small_button(tl!("Open")).clicked() {
                            action = Some(Command::OpenSharedDocument { id: document.id, name: document.name.clone() });
                        }
                        ui.end_row();
                    }
                });
            }
        }
    }
    if let Some(action) = action {
        app.connected_send(action);
    }
    close
}

pub(crate) fn ai_dialog_body(ui: &mut egui::Ui, app: &mut EasyPdfApp, t: &crate::theme::Tokens) -> bool {
    let title = match app.connected.ai_task {
        AiTask::Summary => tl!("Summarize"),
        AiTask::Ask => tl!("Ask about this document"),
        AiTask::Translate => tl!("Translate"),
    };
    ui.label(egui::RichText::new(title).font(crate::theme::semibold(18.0)));
    match app.connected.ai_task {
        AiTask::Ask => {
            ui.add(crate::widgets::block(&mut app.connected.ai_question).desired_rows(3).hint_text(tl!("Question")));
        }
        AiTask::Translate => {
            ui.horizontal(|ui| {
                ui.label(tl!("Translate to"));
                ui.add(crate::widgets::line(&mut app.connected.ai_language));
            });
        }
        AiTask::Summary => {
            ui.label(tl!("Create a concise summary of the open document."));
        }
    }
    if app.connected.busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tl!("Waiting for AI provider…"));
        });
    }
    if let Some(error) = &app.connected.error {
        ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(0xD1, 0x3B, 0x3B)));
    }
    if !app.connected.ai_result.is_empty() {
        ui.separator();
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.label(&app.connected.ai_result);
        });
    }
    ui.add_space(10.0);
    let mut close = false;
    ui.horizontal(|ui| {
        let ready = !app.connected.busy
            && app.connected.snapshot.as_ref().is_some_and(|snapshot| !snapshot.ai.model.is_empty())
            && (app.connected.ai_task != AiTask::Ask || !app.connected.ai_question.trim().is_empty());
        if ui.add_enabled_ui(ready, |ui| crate::widgets::pill_button(ui, tl!("Run"), true)).inner.clicked() {
            app.run_ai();
        }
        if crate::widgets::pill_button(ui, tl!("Close"), false).clicked() {
            close = true;
        }
    });
    if !app.connected.snapshot.as_ref().is_some_and(|snapshot| !snapshot.ai.model.is_empty()) {
        ui.label(egui::RichText::new(tl!("Configure an AI provider in Preferences first.")).small().color(t.text_muted));
    }
    close
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(title).font(crate::theme::semibold(14.0)));
}

fn finish(ui: &mut egui::Ui) -> bool {
    ui.add_space(12.0);
    crate::widgets::pill_button(ui, tl!("OK"), true).clicked()
}

fn account_role_label(role: &str) -> &'static str {
    match role {
        "super_admin" => tl!("Super Admin"),
        "admin" => tl!("Administrator"),
        _ => tl!("User"),
    }
}

fn approval_label(status: &str, active: bool) -> &'static str {
    match status {
        "pending" => tl!("Pending approval"),
        "rejected" => tl!("Rejected"),
        "banned" => tl!("Banned"),
        _ if active => tl!("Active"),
        _ => tl!("Disabled"),
    }
}

fn valid_organization_slug(slug: &str) -> bool {
    let bytes = slug.as_bytes();
    (3..=80).contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn management_modals(ctx: &egui::Context, app: &mut EasyPdfApp, t: &crate::theme::Tokens) -> Option<Command> {
    let snapshot = app.connected.snapshot.clone()?;
    let mut command = None;

    if let Some(mut editor) = app.connected.user_editor.clone() {
        let is_self = editor.id == snapshot.account.id;
        let mut close = false;
        let modal = crate::widgets::show_modal(ctx, "account-edit-user", |ui| {
            ui.set_width(600.0);
            ui.label(egui::RichText::new(tl!("Edit user")).font(crate::theme::semibold(18.0)));
            ui.add_space(8.0);
            ui.add(crate::widgets::line(&mut editor.display_name).hint_text(tl!("Name")).desired_width(f32::INFINITY));
            ui.add(crate::widgets::line(&mut editor.email).hint_text(tl!("Email")).desired_width(f32::INFINITY));
            ui.add_enabled_ui(!is_self, |ui| {
                ui.horizontal(|ui| {
                    craft_ui::ComboBox::from_id_salt("edit-user-role").selected_text(account_role_label(&editor.role)).show_ui(ui, |ui| {
                        ui.selectable_value(&mut editor.role, "user".into(), tl!("User"));
                        ui.selectable_value(&mut editor.role, "admin".into(), tl!("Administrator"));
                        if snapshot.account.role == "super_admin" {
                            ui.selectable_value(&mut editor.role, "super_admin".into(), tl!("Super Admin"));
                        }
                    });
                    if snapshot.account.role == "super_admin" && editor.role != "super_admin" {
                        craft_ui::ComboBox::from_id_salt("edit-user-organization")
                            .selected_text(
                                snapshot
                                    .organizations
                                    .iter()
                                    .find(|organization| Some(organization.id) == editor.organization_id)
                                    .map_or_else(|| tl!("Personal account").to_string(), |organization| organization.name.clone()),
                            )
                            .show_ui(ui, |ui| {
                                if editor.role == "user" {
                                    ui.selectable_value(&mut editor.organization_id, None, tl!("Personal account"));
                                }
                                for organization in &snapshot.organizations {
                                    ui.selectable_value(&mut editor.organization_id, Some(organization.id), &organization.name);
                                }
                            });
                    }
                    craft_ui::ComboBox::from_id_salt("edit-user-approval")
                        .selected_text(approval_label(&editor.approval_status, editor.approval_status == "approved"))
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                ("pending", tl!("Pending approval")),
                                ("approved", tl!("Active")),
                                ("rejected", tl!("Rejected")),
                                ("banned", tl!("Banned")),
                            ] {
                                ui.selectable_value(&mut editor.approval_status, value.into(), label);
                            }
                        });
                });
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(tl!("Leave both password fields blank to keep the current password.")).small().color(t.text_muted));
            ui.horizontal(|ui| {
                crate::widgets::password_line(ui, &mut editor.password, egui::Id::unique("edit-user-password"), tl!("New password"), 250.0);
                crate::widgets::password_line(
                    ui,
                    &mut editor.password_confirm,
                    egui::Id::unique("edit-user-password-confirm"),
                    tl!("Confirm Password"),
                    250.0,
                );
            });
            let password_valid = editor.password.is_empty() && editor.password_confirm.is_empty()
                || editor.password.len() >= 8 && editor.password == editor.password_confirm;
            if !editor.password.is_empty() || !editor.password_confirm.is_empty() {
                let (message, color) = if editor.password != editor.password_confirm {
                    (tl!("Passwords do not match"), egui::Color32::from_rgb(0xD1, 0x3B, 0x3B))
                } else if editor.password.len() < 8 {
                    (tl!("Password must contain at least 8 characters"), egui::Color32::from_rgb(0xD1, 0x3B, 0x3B))
                } else {
                    (tl!("Passwords match"), egui::Color32::from_rgb(0x35, 0xA8, 0x53))
                };
                ui.label(egui::RichText::new(message).small().color(color));
            }
            let role_valid = editor.role == "super_admin" || editor.role == "user" || editor.organization_id.is_some();
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled_ui(!editor.display_name.trim().is_empty() && !editor.email.trim().is_empty() && password_valid && role_valid, |ui| {
                        crate::widgets::pill_button(ui, tl!("Save changes"), true)
                    })
                    .inner
                    .clicked()
                {
                    command = Some(Command::UpdateUser {
                        id: editor.id,
                        display_name: editor.display_name.trim().to_string(),
                        email: editor.email.trim().to_string(),
                        password: editor.password.clone(),
                        role: editor.role.clone(),
                        organization_id: if editor.role == "super_admin" { None } else { editor.organization_id },
                        approval_status: editor.approval_status.clone(),
                    });
                    close = true;
                }
                if crate::widgets::pill_button(ui, tl!("Cancel"), false).clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            app.connected.user_editor = None;
        } else {
            app.connected.user_editor = Some(editor);
        }
    } else if app.connected.confirm_delete_users {
        let mut close = false;
        let selected: Vec<u64> = app.connected.selected_user_ids.iter().copied().collect();
        let modal = crate::widgets::show_modal(ctx, "account-delete-users", |ui| {
            ui.set_width(460.0);
            ui.label(egui::RichText::new(tl!("Delete selected users?")).font(crate::theme::semibold(18.0)));
            ui.label(tl!("This action cannot be undone."));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::widgets::pill_button(ui, tl!("Delete"), true).clicked() {
                    command = Some(Command::DeleteUsers { ids: selected.clone() });
                    close = true;
                }
                if crate::widgets::pill_button(ui, tl!("Cancel"), false).clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            app.connected.confirm_delete_users = false;
            if command.is_some() {
                app.connected.selected_user_ids.clear();
            }
        }
    } else if let Some(mut editor) = app.connected.organization_editor.clone() {
        let mut close = false;
        let modal = crate::widgets::show_modal(ctx, "account-edit-organization", |ui| {
            ui.set_width(500.0);
            ui.label(egui::RichText::new(tl!("Edit organization")).font(crate::theme::semibold(18.0)));
            ui.add(crate::widgets::line(&mut editor.name).hint_text(tl!("Organization name")).desired_width(f32::INFINITY));
            ui.add(crate::widgets::line(&mut editor.slug).hint_text("slug-azienda").desired_width(f32::INFINITY));
            ui.add(crate::widgets::checkbox(&mut editor.is_active, tl!("Active")));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled_ui(!editor.name.trim().is_empty() && valid_organization_slug(editor.slug.trim()), |ui| {
                        crate::widgets::pill_button(ui, tl!("Save changes"), true)
                    })
                    .inner
                    .clicked()
                {
                    command = Some(Command::UpdateOrganization {
                        id: editor.id,
                        name: editor.name.trim().to_string(),
                        slug: editor.slug.trim().to_string(),
                        is_active: editor.is_active,
                    });
                    close = true;
                }
                if crate::widgets::pill_button(ui, tl!("Cancel"), false).clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            app.connected.organization_editor = None;
        } else {
            app.connected.organization_editor = Some(editor);
        }
    } else if let Some(id) = app.connected.delete_organization_id {
        let name =
            snapshot.organizations.iter().find(|organization| organization.id == id).map(|organization| organization.name.as_str()).unwrap_or("");
        let mut close = false;
        let modal = crate::widgets::show_modal(ctx, "account-delete-organization", |ui| {
            ui.set_width(540.0);
            ui.label(egui::RichText::new(tl!("Delete organization")).font(crate::theme::semibold(18.0)));
            ui.label(name);
            crate::widgets::radio_value(ui, &mut app.connected.delete_organization_users, false, tl!("Detach users and make them personal accounts"));
            crate::widgets::radio_value(ui, &mut app.connected.delete_organization_users, true, tl!("Delete organization and all its users"));
            ui.label(egui::RichText::new(tl!("Organization data and cover kits will be deleted.")).small().color(t.text_muted));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::widgets::pill_button(ui, tl!("Delete"), true).clicked() {
                    command = Some(Command::DeleteOrganization { id, delete_users: app.connected.delete_organization_users });
                    close = true;
                }
                if crate::widgets::pill_button(ui, tl!("Cancel"), false).clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            app.connected.delete_organization_id = None;
            if command.is_some() {
                app.connected.selected_organization_ids.clear();
            }
        }
    }
    command
}

/// Shared non-general Preferences pages. Returns `true` when Preferences should close.
#[allow(clippy::cognitive_complexity, reason = "Release parity: split the connected preferences state machine in a separate refactor")]
pub(crate) fn preferences_body(ui: &mut egui::Ui, app: &mut EasyPdfApp, t: &crate::theme::Tokens) -> bool {
    let mut action = None;
    let mut pick_cover = None;
    let mut clear_cover_draft = false;
    let mut close = false;
    if let Some(error) = &app.connected.error {
        ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(0xD1, 0x3B, 0x3B)));
    }
    match app.connected.tab {
        PreferencesTab::General => {}
        PreferencesTab::Account => {
            if let Some(snapshot) = app.connected.snapshot.clone() {
                let account = snapshot.account;
                ui.label(egui::RichText::new(account.display_name).font(crate::theme::semibold(15.0)));
                ui.label(&account.email);
                ui.label(account.organization_name.as_deref().unwrap_or(tl!("Personal account")));
                if ui.add_enabled_ui(!app.connected.busy, |ui| crate::widgets::pill_button(ui, tl!("Synchronize"), false)).inner.clicked() {
                    action = Some(Command::Refresh);
                }
                if !app.portal_url.is_empty() {
                    section(ui, tl!("Web installation"));
                    ui.label(
                        egui::RichText::new(tl!("Enter the address of the easyPDF Web installation used by your company."))
                            .small()
                            .color(t.text_muted),
                    );
                    ui.add(crate::widgets::line(&mut app.portal_url).hint_text("https://easypdf.example.it"));
                    ui.add(crate::widgets::line(&mut app.portal_email).hint_text(tl!("Email")));
                    crate::widgets::password_line(
                        ui,
                        &mut app.connected.password,
                        egui::Id::unique("portal-password-connected"),
                        tl!("Password"),
                        f32::INFINITY,
                    );
                    let ready = (app.portal_url.starts_with("http://") || app.portal_url.starts_with("https://"))
                        && !app.portal_email.trim().is_empty()
                        && !app.connected.password.is_empty();
                    if ui.add_enabled_ui(ready, |ui| crate::widgets::pill_button(ui, tl!("Connect to this site"), true)).inner.clicked() {
                        action = Some(Command::Connect {
                            url: app.portal_url.trim_end_matches('/').to_string(),
                            email: app.portal_email.trim().to_string(),
                            password: std::mem::take(&mut app.connected.password),
                        });
                    }
                }
                if account.can_manage_accounts {
                    section(ui, tl!("Users"));
                    ui.horizontal(|ui| {
                        let one_selected = app.connected.selected_user_ids.len() == 1;
                        if ui.add_enabled_ui(one_selected, |ui| crate::widgets::pill_button(ui, tl!("Modify"), false)).inner.clicked()
                            && let Some(id) = app.connected.selected_user_ids.iter().next().copied()
                            && let Some(user) = snapshot.users.iter().find(|user| user.id == id)
                        {
                            app.connected.user_editor = Some(UserEditor {
                                id: user.id,
                                display_name: user.display_name.clone(),
                                email: user.email.clone(),
                                role: user.role.clone(),
                                organization_id: user.organization_id,
                                approval_status: user.approval_status.clone(),
                                ..Default::default()
                            });
                        }
                        let may_delete = !app.connected.selected_user_ids.is_empty() && !app.connected.selected_user_ids.contains(&account.id);
                        if ui.add_enabled_ui(may_delete, |ui| crate::widgets::pill_button(ui, tl!("Delete"), false)).inner.clicked() {
                            app.connected.confirm_delete_users = true;
                        }
                    });
                    egui::ScrollArea::horizontal().id_salt("account-users-table-scroll").show(ui, |ui| {
                        egui::Grid::new("account-users-table").striped(true).min_col_width(96.0).show(ui, |ui| {
                            ui.label(egui::RichText::new(tl!("Select")).strong());
                            for heading in ["Name", "Email", "Role", "Organization", "Status", "Actions"] {
                                ui.label(egui::RichText::new(tl!(heading)).strong());
                            }
                            ui.end_row();
                            for user in &snapshot.users {
                                let mut selected = app.connected.selected_user_ids.contains(&user.id);
                                if ui.add(crate::widgets::checkbox_without_text(&mut selected)).changed() {
                                    if selected {
                                        app.connected.selected_user_ids.insert(user.id);
                                    } else {
                                        app.connected.selected_user_ids.remove(&user.id);
                                    }
                                }
                                ui.label(&user.display_name);
                                ui.label(&user.email);
                                ui.label(account_role_label(&user.role));
                                ui.label(user.organization_name.as_deref().unwrap_or(tl!("Personal account")));
                                let status_color = if user.approval_status == "approved" && user.is_active { t.text } else { t.accent };
                                ui.label(egui::RichText::new(approval_label(&user.approval_status, user.is_active)).color(status_color));
                                ui.horizontal(|ui| {
                                    if user.id == account.id {
                                        ui.label(egui::RichText::new(tl!("Current account")).small().color(t.text_muted));
                                        return;
                                    }
                                    if user.approval_status != "approved" && ui.small_button(tl!("Approve")).clicked() {
                                        action = Some(Command::SetUserApproval { id: user.id, status: "approved".into() });
                                    }
                                    if user.approval_status == "pending" && ui.small_button(tl!("Reject")).clicked() {
                                        action = Some(Command::SetUserApproval { id: user.id, status: "rejected".into() });
                                    }
                                    if user.approval_status == "banned" {
                                        if ui.small_button(tl!("Unban")).clicked() {
                                            action = Some(Command::SetUserApproval { id: user.id, status: "approved".into() });
                                        }
                                    } else if ui.small_button(tl!("Ban")).clicked() {
                                        action = Some(Command::SetUserApproval { id: user.id, status: "banned".into() });
                                    }
                                });
                                ui.end_row();
                            }
                        });
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add(crate::widgets::line(&mut app.connected.new_user_name).hint_text(tl!("Name")));
                        ui.add(crate::widgets::line(&mut app.connected.new_user_email).hint_text(tl!("Email")));
                    });
                    ui.horizontal(|ui| {
                        if account.role == "super_admin" || account.role == "admin" {
                            craft_ui::ComboBox::from_id_salt("new-user-role")
                                .selected_text(tl!(account_role_label(&app.connected.new_user_role)))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut app.connected.new_user_role, "user".into(), tl!("User"));
                                    ui.selectable_value(&mut app.connected.new_user_role, "admin".into(), tl!("Administrator"));
                                    if account.role == "super_admin" {
                                        ui.selectable_value(&mut app.connected.new_user_role, "super_admin".into(), tl!("Super Admin"));
                                    }
                                });
                        } else {
                            ui.label(tl!("User"));
                        }
                        if account.role == "super_admin" && app.connected.new_user_role != "super_admin" {
                            craft_ui::ComboBox::from_id_salt("new-user-org")
                                .selected_text(
                                    snapshot
                                        .organizations
                                        .iter()
                                        .find(|organization| Some(organization.id) == app.connected.new_user_organization)
                                        .map_or_else(|| tl!("Personal account").to_string(), |organization| organization.name.clone()),
                                )
                                .show_ui(ui, |ui| {
                                    if app.connected.new_user_role == "user" {
                                        ui.selectable_value(&mut app.connected.new_user_organization, None, tl!("Personal account"));
                                    }
                                    for organization in &snapshot.organizations {
                                        ui.selectable_value(&mut app.connected.new_user_organization, Some(organization.id), &organization.name);
                                    }
                                });
                        } else if account.role == "admin" {
                            ui.label(account.organization_name.as_deref().unwrap_or(tl!("Personal account")));
                        }
                    });
                    ui.horizontal(|ui| {
                        crate::widgets::password_line(
                            ui,
                            &mut app.connected.new_user_password,
                            egui::Id::unique("new-user-password"),
                            tl!("Initial password"),
                            250.0,
                        );
                        crate::widgets::password_line(
                            ui,
                            &mut app.connected.new_user_password_confirm,
                            egui::Id::unique("new-user-password-confirm"),
                            tl!("Confirm Password"),
                            250.0,
                        );
                    });
                    ui.label(egui::RichText::new(tl!("Password must contain at least 8 characters")).small().color(t.text_muted));
                    let new_passwords_match =
                        !app.connected.new_user_password.is_empty() && app.connected.new_user_password == app.connected.new_user_password_confirm;
                    if !app.connected.new_user_password_confirm.is_empty() {
                        let (message, color) = if !new_passwords_match {
                            (tl!("Passwords do not match"), egui::Color32::from_rgb(0xD1, 0x3B, 0x3B))
                        } else if app.connected.new_user_password.len() < 8 {
                            (tl!("Passwords match, but the password is too short"), egui::Color32::from_rgb(0xD1, 0x3B, 0x3B))
                        } else {
                            (tl!("Passwords match"), egui::Color32::from_rgb(0x35, 0xA8, 0x53))
                        };
                        ui.label(egui::RichText::new(message).small().color(color));
                    }
                    if ui
                        .add_enabled_ui(
                            !app.connected.new_user_name.trim().is_empty()
                                && !app.connected.new_user_email.trim().is_empty()
                                && app.connected.new_user_password.len() >= 8
                                && new_passwords_match
                                && (app.connected.new_user_role != "admin" || app.connected.new_user_organization.is_some()),
                            |ui| crate::widgets::pill_button(ui, tl!("Create user"), true),
                        )
                        .inner
                        .clicked()
                    {
                        action = Some(Command::CreateUser {
                            display_name: app.connected.new_user_name.trim().to_string(),
                            email: app.connected.new_user_email.trim().to_string(),
                            password: std::mem::take(&mut app.connected.new_user_password),
                            role: app.connected.new_user_role.clone(),
                            organization_id: app.connected.new_user_organization,
                            organization_role: app
                                .connected
                                .new_user_organization
                                .map(|_| if app.connected.new_user_role == "admin" { "admin".into() } else { "member".into() }),
                        });
                        app.connected.new_user_password_confirm.clear();
                    }
                    section(ui, tl!("Organizations"));
                    if account.role == "super_admin" {
                        ui.horizontal(|ui| {
                            let one_selected = app.connected.selected_organization_ids.len() == 1;
                            if ui.add_enabled_ui(one_selected, |ui| crate::widgets::pill_button(ui, tl!("Modify"), false)).inner.clicked()
                                && let Some(id) = app.connected.selected_organization_ids.iter().next().copied()
                                && let Some(organization) = snapshot.organizations.iter().find(|organization| organization.id == id)
                            {
                                app.connected.organization_editor = Some(OrganizationEditor {
                                    id: organization.id,
                                    name: organization.name.clone(),
                                    slug: organization.slug.clone(),
                                    is_active: organization.is_active,
                                });
                            }
                            if ui.add_enabled_ui(one_selected, |ui| crate::widgets::pill_button(ui, tl!("Delete"), false)).inner.clicked() {
                                app.connected.delete_organization_id = app.connected.selected_organization_ids.iter().next().copied();
                                app.connected.delete_organization_users = false;
                            }
                        });
                    }
                    egui::Grid::new("account-organizations-table").striped(true).min_col_width(100.0).show(ui, |ui| {
                        if account.role == "super_admin" {
                            ui.label(egui::RichText::new(tl!("Select")).strong());
                        }
                        for heading in ["Name", "Slug", "Members", "Status", "Actions"] {
                            ui.label(egui::RichText::new(tl!(heading)).strong());
                        }
                        ui.end_row();
                        for organization in &snapshot.organizations {
                            if account.role == "super_admin" {
                                let mut selected = app.connected.selected_organization_ids.contains(&organization.id);
                                if ui.add(crate::widgets::checkbox_without_text(&mut selected)).changed() {
                                    if selected {
                                        app.connected.selected_organization_ids.insert(organization.id);
                                    } else {
                                        app.connected.selected_organization_ids.remove(&organization.id);
                                    }
                                }
                            }
                            ui.label(&organization.name);
                            ui.label(&organization.slug);
                            ui.label(organization.member_count.to_string());
                            ui.label(if organization.is_active { tl!("Active") } else { tl!("Disabled") });
                            if account.role == "super_admin" {
                                let label = if organization.is_active { tl!("Disable") } else { tl!("Enable") };
                                if ui.small_button(label).clicked() {
                                    action = Some(Command::ToggleOrganization { id: organization.id });
                                }
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
                    if account.role == "super_admin" {
                        ui.horizontal(|ui| {
                            ui.add(crate::widgets::line(&mut app.connected.new_organization_name).hint_text(tl!("Organization name")));
                            ui.add(crate::widgets::line(&mut app.connected.new_organization_slug).hint_text("slug-azienda"));
                            let valid = !app.connected.new_organization_name.trim().is_empty()
                                && valid_organization_slug(app.connected.new_organization_slug.trim());
                            if ui.add_enabled_ui(valid, |ui| crate::widgets::pill_button(ui, tl!("Create organization"), true)).inner.clicked() {
                                action = Some(Command::CreateOrganization {
                                    name: app.connected.new_organization_name.trim().to_string(),
                                    slug: app.connected.new_organization_slug.trim().to_string(),
                                });
                            }
                        });
                    }
                }
            } else {
                ui.label(tl!("Connect the desktop app to your easyPDF Web installation."));
                ui.add(crate::widgets::line(&mut app.portal_url).hint_text("https://pdf.example.it"));
                ui.add(crate::widgets::line(&mut app.portal_email).hint_text(tl!("Email")));
                crate::widgets::password_line(
                    ui,
                    &mut app.connected.password,
                    egui::Id::unique("portal-password-disconnected"),
                    tl!("Password"),
                    f32::INFINITY,
                );
                let ready = app.portal_url.starts_with("http://") || app.portal_url.starts_with("https://");
                if ui
                    .add_enabled_ui(ready && !app.portal_email.trim().is_empty() && !app.connected.password.is_empty(), |ui| {
                        crate::widgets::pill_button(ui, tl!("Connect"), true)
                    })
                    .inner
                    .clicked()
                {
                    action = Some(Command::Connect {
                        url: app.portal_url.trim_end_matches('/').to_string(),
                        email: app.portal_email.trim().to_string(),
                        password: std::mem::take(&mut app.connected.password),
                    });
                }
                ui.label(
                    egui::RichText::new(tl!("URL and email stay on this device. Password and access token stay only in memory."))
                        .small()
                        .color(t.text_muted),
                );
            }
        }
        PreferencesTab::Covers => {
            if let Some(snapshot) = app.connected.snapshot.clone() {
                ui.label(tl!("Cover kits are shared with every account in the organization."));
                if snapshot.account.can_manage {
                    section(ui, tl!("Create cover kit"));
                    for (purpose, label, cover) in [
                        (crate::files::FilePurpose::CoverFront, tl!("Front cover"), app.cover_draft.front.as_ref()),
                        (crate::files::FilePurpose::CoverBack, tl!("Back cover"), app.cover_draft.back.as_ref()),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(label).strong());
                            ui.label(egui::RichText::new(cover.map_or(tl!("Not selected"), |item| item.name.as_str())).color(t.text_muted));
                            if crate::widgets::pill_button(ui, tl!("Choose PDF…"), false).clicked() {
                                pick_cover = Some(purpose);
                            }
                        });
                    }
                    ui.add(crate::widgets::line(&mut app.connected.new_kit_name).hint_text(tl!("Cover kit name")));
                    ui.add(crate::widgets::checkbox(&mut app.cover_draft.make_default, tl!("Use as default cover kit")));
                    let ready = !app.connected.busy
                        && !app.connected.new_kit_name.trim().is_empty()
                        && (app.cover_draft.front.is_some() || app.cover_draft.back.is_some());
                    if ui.add_enabled_ui(ready, |ui| crate::widgets::pill_button(ui, tl!("Save cover kit"), true)).inner.clicked() {
                        action = Some(Command::SaveCoverKit {
                            name: app.connected.new_kit_name.trim().to_string(),
                            make_default: app.cover_draft.make_default,
                            front_name: app.cover_draft.front.as_ref().map(|cover| cover.name.clone()),
                            front: app.cover_draft.front.as_ref().map(|cover| cover.bytes.as_ref().clone()),
                            back_name: app.cover_draft.back.as_ref().map(|cover| cover.name.clone()),
                            back: app.cover_draft.back.as_ref().map(|cover| cover.bytes.as_ref().clone()),
                        });
                        clear_cover_draft = true;
                    }
                } else {
                    ui.label(egui::RichText::new(tl!("Only organization administrators can manage cover kits.")).small().color(t.text_muted));
                }
                close = finish(ui);
                section(ui, tl!("Saved cover kits"));
                if snapshot.account.can_manage {
                    let selected: Vec<u64> = app.connected.selected_cover_ids.iter().copied().collect();
                    if ui
                        .add_enabled_ui(!selected.is_empty() && !app.connected.busy, |ui| {
                            crate::widgets::pill_button(ui, tl!("Delete selected"), false)
                        })
                        .inner
                        .clicked()
                    {
                        action = Some(Command::DeleteCovers { ids: selected });
                        app.connected.selected_cover_ids.clear();
                    }
                }
                if snapshot.cover_kits.is_empty() {
                    ui.label(egui::RichText::new(tl!("No cover kits saved yet.")).color(t.text_muted));
                } else {
                    egui::Grid::new("cover-kit-table").striped(true).min_col_width(90.0).show(ui, |ui| {
                        ui.label(tl!("Select"));
                        ui.label(egui::RichText::new(tl!("Name")).strong());
                        ui.label(egui::RichText::new(tl!("Front preview")).strong());
                        ui.label(egui::RichText::new(tl!("Back preview")).strong());
                        ui.label("");
                        ui.end_row();
                        for kit in &snapshot.cover_kits {
                            let mut selected = app.connected.selected_cover_ids.contains(&kit.id);
                            if ui.add_enabled(snapshot.account.can_manage, crate::widgets::checkbox(&mut selected, "")).changed() {
                                if selected {
                                    app.connected.selected_cover_ids.insert(kit.id);
                                } else {
                                    app.connected.selected_cover_ids.remove(&kit.id);
                                }
                            }
                            let default = if kit.is_default { format!(" · {}", tl!("Default")) } else { String::new() };
                            ui.label(format!("{}{}", kit.name, default));
                            cover_preview(ui, kit.id, "front", kit.front_preview.as_deref(), kit.front_name.as_deref(), t);
                            cover_preview(ui, kit.id, "back", kit.back_preview.as_deref(), kit.back_name.as_deref(), t);
                            if snapshot.account.can_manage && !kit.is_default && ui.small_button(tl!("Set as default")).clicked() {
                                action = Some(Command::SetDefaultCover { id: kit.id });
                            }
                            ui.end_row();
                        }
                    });
                }
            } else {
                ui.label(tl!("Connect your account to synchronize cover kits."));
            }
        }
        PreferencesTab::Ai => {
            let snapshot = app.connected.snapshot.clone();
            let shared = snapshot.as_ref().and_then(|value| value.shared_ai.clone());
            ui.label(tl!("AI configuration to use"));
            if shared.is_some() {
                let previous_source = app.connected.ai_source.clone();
                craft_ui::ComboBox::from_id_salt("ai-source")
                    .selected_text(if app.connected.ai_source == "organization" { tl!("Organization AI") } else { tl!("Personal AI") })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.connected.ai_source, "personal".into(), tl!("Personal AI"));
                        ui.selectable_value(&mut app.connected.ai_source, "organization".into(), tl!("Organization AI"));
                    });
                if app.connected.ai_source != previous_source {
                    let settings =
                        if app.connected.ai_source == "organization" { shared.as_ref() } else { snapshot.as_ref().map(|value| &value.personal_ai) };
                    if let Some(settings) = settings {
                        app.connected.ai_provider = settings.provider.clone();
                        app.connected.ai_auth_mode = settings.auth_mode.clone();
                        app.connected.ai_base_url = settings.base_url.clone();
                        app.connected.ai_model = settings.model.clone();
                        app.connected.ai_secret.clear();
                        app.connected.ai_models.clear();
                    }
                    action = Some(Command::SetAiSource { source: app.connected.ai_source.clone() });
                }
            } else {
                app.connected.ai_source = "personal".into();
                ui.label(egui::RichText::new(tl!("No organization AI has been shared with this account.")).small().color(t.text_muted));
            }

            if app.connected.ai_source == "organization" {
                if let Some(settings) = shared {
                    section(ui, tl!("Organization AI"));
                    ui.label(format!(
                        "{}: {}",
                        tl_ctx!("organization ai", "Shared by"),
                        settings.owner_name.as_deref().unwrap_or(tl!("Administrator"))
                    ));
                    ui.label(format!("{}: {}", tl!("AI provider"), settings.provider));
                    ui.label(format!("{}: {}", tl!("Model"), settings.model));
                    ui.label(
                        egui::RichText::new(tl!("The credential stays encrypted on the server and is never shown to organization users."))
                            .small()
                            .color(t.text_muted),
                    );
                }
            } else {
                section(ui, tl!("Personal AI"));
                ui.label(tl!("AI provider"));
                let previous_provider = app.connected.ai_provider.clone();
                craft_ui::ComboBox::from_id_salt("ai-provider").selected_text(&app.connected.ai_provider).show_ui(ui, |ui| {
                    for (id, label) in
                        [("openai", "OpenAI / ChatGPT"), ("gemini", "Gemini"), ("openrouter", "OpenRouter"), ("compatible", "OpenAI-compatible")]
                    {
                        ui.selectable_value(&mut app.connected.ai_provider, id.into(), label);
                    }
                });
                if app.connected.ai_provider != previous_provider {
                    app.connected.ai_models.clear();
                    app.connected.ai_model.clear();
                    app.connected.ai_secret.clear();
                }
                if app.connected.ai_provider == "openai" {
                    craft_ui::ComboBox::from_id_salt("ai-auth").selected_text(&app.connected.ai_auth_mode).show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.connected.ai_auth_mode, "api_key".into(), tl!("API key"));
                        ui.selectable_value(&mut app.connected.ai_auth_mode, "chatgpt_oauth".into(), "ChatGPT OAuth");
                    });
                } else {
                    app.connected.ai_auth_mode = "api_key".into();
                }
                if app.connected.ai_auth_mode == "chatgpt_oauth" {
                    if crate::widgets::pill_button(ui, tl!("Continue with ChatGPT"), true).clicked() {
                        action = Some(Command::StartChatGptAuth);
                    }
                    ui.label(egui::RichText::new(tl!("Browser opens. Login. Allow access. Return here.")).small().color(t.text_muted));
                } else {
                    crate::widgets::password_line(
                        ui,
                        &mut app.connected.ai_secret,
                        egui::Id::unique("ai-provider-secret"),
                        tl!("API key"),
                        f32::INFINITY,
                    );
                }
                if app.connected.ai_provider == "compatible" {
                    ui.add(crate::widgets::line(&mut app.connected.ai_base_url).hint_text("https://server.example/v1"));
                }
                let saved_credential = snapshot.as_ref().is_some_and(|value| {
                    value.personal_ai.provider == app.connected.ai_provider
                        && value.personal_ai.auth_mode == app.connected.ai_auth_mode
                        && value.personal_ai.has_secret
                });
                let can_load_models = !app.connected.busy
                    && if app.connected.ai_auth_mode == "chatgpt_oauth" {
                        saved_credential
                    } else {
                        !app.connected.ai_secret.is_empty() || saved_credential || app.connected.ai_provider == "compatible"
                    };
                ui.horizontal(|ui| {
                    craft_ui::ComboBox::from_id_salt("ai-model")
                        .selected_text(if app.connected.ai_model.is_empty() { tl!("Choose model") } else { &app.connected.ai_model })
                        .show_ui(ui, |ui| {
                            for model in &app.connected.ai_models {
                                ui.selectable_value(&mut app.connected.ai_model, model.clone(), model);
                            }
                        });
                    if ui.add_enabled_ui(can_load_models, |ui| crate::widgets::pill_button(ui, tl!("Load models"), false)).inner.clicked() {
                        action = Some(Command::LoadModels {
                            source: "personal".into(),
                            provider: app.connected.ai_provider.clone(),
                            auth_mode: app.connected.ai_auth_mode.clone(),
                            base_url: app.connected.ai_base_url.clone(),
                            secret: app.connected.ai_secret.clone(),
                        });
                    }
                });
                if let Some(value) = &snapshot
                    && value.account.role == "admin"
                    && value.account.organization_id.is_some()
                {
                    section(ui, tl!("Share with organization users"));
                    ui.label(
                        egui::RichText::new(tl!("Selected users may choose this provider or keep their personal AI.")).small().color(t.text_muted),
                    );
                    for user in value.users.iter().filter(|user| user.id != value.account.id && user.is_active && user.approval_status == "approved")
                    {
                        let mut selected = app.connected.ai_shared_user_ids.contains(&user.id);
                        if ui.add(crate::widgets::checkbox(&mut selected, format!("{} ({})", user.display_name, user.email))).changed() {
                            if selected {
                                app.connected.ai_shared_user_ids.insert(user.id);
                            } else {
                                app.connected.ai_shared_user_ids.remove(&user.id);
                            }
                        }
                    }
                }
                if crate::widgets::pill_button(ui, tl!("Save AI provider"), true).clicked() {
                    action = Some(Command::SaveAi {
                        provider: app.connected.ai_provider.clone(),
                        auth_mode: app.connected.ai_auth_mode.clone(),
                        base_url: app.connected.ai_base_url.clone(),
                        model: app.connected.ai_model.clone(),
                        secret: std::mem::take(&mut app.connected.ai_secret),
                        shared_user_ids: app.connected.ai_shared_user_ids.iter().copied().collect(),
                    });
                }
            }
            let help = match app.connected.ai_provider.as_str() {
                "gemini" => tl!("Google AI Studio. Create key. Paste key. Load models. Choose model."),
                "openrouter" => tl!("OpenRouter. Create key. Paste key. Load models. Choose model."),
                "compatible" => tl!("Enter /v1 URL. Paste key if required. Load models. Choose model."),
                _ if app.connected.ai_auth_mode == "chatgpt_oauth" => tl!("Press Continue. Login to ChatGPT. Allow access. Done."),
                _ => tl!("OpenAI Platform. Create API key. Paste key. Load models. Choose model."),
            };
            ui.horizontal_wrapped(|ui| {
                ui.add(crate::icons::image("info", 16.0, t.accent));
                ui.label(egui::RichText::new(help).small().color(t.text_muted));
            });
        }
    }
    if let Some(modal_action) = management_modals(ui.ctx(), app, t) {
        action = Some(modal_action);
    }
    if let Some(purpose) = pick_cover {
        app.pick_files(purpose, false);
    }
    if let Some(action) = action {
        app.connected_send(action);
    }
    if clear_cover_draft {
        app.cover_draft = Default::default();
        app.connected.new_kit_name.clear();
    }
    if app.connected.tab == PreferencesTab::Covers { close } else { finish(ui) }
}

fn cover_preview(ui: &mut egui::Ui, id: u64, side: &str, encoded: Option<&str>, name: Option<&str>, t: &crate::theme::Tokens) {
    use base64::Engine as _;
    if let Some(bytes) = encoded.and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok()) {
        ui.add(egui::Image::from_bytes(format!("bytes://cover-{id}-{side}.png"), bytes).fit_to_exact_size(egui::vec2(54.0, 72.0)));
    } else {
        ui.vertical(|ui| {
            ui.add(crate::icons::image("file-text", 22.0, t.accent));
            ui.label(egui::RichText::new(name.unwrap_or(tl!("Not present"))).small().color(t.text_muted));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_selects_default_cover_and_never_keeps_provider_secret() {
        let mut state = State { ai_secret: "transient".into(), ..Default::default() };
        state.seed_snapshot(Snapshot {
            cover_kits: vec![
                CoverKitInfo { id: 4, name: "Other".into(), ..Default::default() },
                CoverKitInfo { id: 7, name: "Default".into(), is_default: true, ..Default::default() },
            ],
            ai: AiSettings { provider: "gemini".into(), model: "gemini-test".into(), has_secret: true, ..Default::default() },
            ..Default::default()
        });
        assert_eq!(state.selected_cover, Some(7));
        assert_eq!(state.ai_provider, "gemini");
        assert_eq!(state.ai_model, "gemini-test");
        assert_eq!(state.ai_secret, "transient", "server snapshots never contain or overwrite the in-memory secret box");
    }

    #[test]
    fn snapshot_without_default_selects_first_cover() {
        let mut state = State::default();
        state.seed_snapshot(Snapshot { cover_kits: vec![CoverKitInfo { id: 9, name: "Only".into(), ..Default::default() }], ..Default::default() });
        assert_eq!(state.selected_cover, Some(9));
    }

    #[test]
    fn refreshed_snapshot_loads_the_default_cover_requested_by_apply_covers() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let commands = Arc::clone(&seen);
        let mut app = EasyPdfApp::new();
        app.connected.load_default_cover_after_refresh = true;
        app.connected_command = Some(Box::new(move |command| {
            if let Command::LoadCoverKit { id } = command {
                match commands.lock() {
                    Ok(mut commands) => commands.push(id),
                    Err(poisoned) => poisoned.into_inner().push(id),
                }
            }
        }));
        let snapshot = Snapshot {
            cover_kits: vec![CoverKitInfo { id: 7, name: "Default".into(), is_default: true, ..Default::default() }],
            ..Default::default()
        };
        match app.connected_events.lock() {
            Ok(mut events) => events.push(Event::Snapshot(snapshot)),
            Err(poisoned) => poisoned.into_inner().push(Event::Snapshot(snapshot)),
        }

        app.poll_connected_events();

        let ids = match seen.lock() {
            Ok(ids) => ids.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        assert_eq!(ids, vec![7]);
        assert_eq!(app.cover_draft.selected_kit, Some(7));
        assert!(!app.connected.load_default_cover_after_refresh);
    }

    #[test]
    fn organization_manager_creates_members_inside_own_organization() {
        let mut state = State { new_user_role: "admin".into(), ..Default::default() };
        state.seed_snapshot(Snapshot {
            account: Account {
                role: "user".into(),
                organization_id: Some(42),
                organization_role: Some("admin".into()),
                can_manage: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(state.new_user_role, "user");
        assert_eq!(state.new_user_organization, Some(42));
    }

    #[test]
    fn downloaded_shared_document_enters_the_normal_open_queue() {
        let mut app = EasyPdfApp::new();
        let bytes = b"%PDF-1.7\n%%EOF\n".to_vec();
        match app.connected_events.lock() {
            Ok(mut events) => events.push(Event::SharedDocumentLoaded { name: "shared.pdf".into(), bytes: bytes.clone() }),
            Err(poisoned) => poisoned.into_inner().push(Event::SharedDocumentLoaded { name: "shared.pdf".into(), bytes: bytes.clone() }),
        }

        app.poll_connected_events();

        let queued = match app.inbox.lock() {
            Ok(inbox) => inbox.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        assert_eq!(queued, vec![("shared.pdf".into(), bytes)]);
        assert!(!app.connected.busy);
    }
}
