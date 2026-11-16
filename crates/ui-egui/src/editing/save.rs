use super::{EasyPdfApp, StagingName, staging_suffixes};
#[cfg(not(target_arch = "wasm32"))]
use crate::file_filter::AddFilterAnyCase as _;

/// A save held back by [`Dialog::OverwriteChanged`](crate::Dialog::OverwriteChanged): the file
/// on disk is another version than the document came from (#2166).
#[cfg(not(target_arch = "wasm32"))]
pub struct PendingOverwrite {
    pub id: easypdf_engine::DocId,
    pub dest: String,
    target: SaveTarget,
    /// What was to run once the document is written (close its tab, quit).
    after: Box<dyn FnOnce(&mut EasyPdfApp) + Send>,
}

/// Where Save writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveTarget {
    /// The file's own path; asks for one if the document has none (e.g. opened from bytes).
    InPlace,
    /// Always ask for a new location.
    As,
}

impl EasyPdfApp {
    /// Save the active document. Returns `true` if it was written.
    pub fn save_active(&mut self, target: SaveTarget) -> bool {
        match self.active {
            Some(i) => self.save_view(i, target),
            None => false,
        }
    }

    /// Save the document shown in tab `index`. Returns `true` if it was written now. When it has
    /// to ask where (a new document, or Save As) it returns `false` and saves on a later frame,
    /// once the user has chosen.
    pub fn save_view(&mut self, index: usize, target: SaveTarget) -> bool {
        self.save_then(index, target, |_| {})
    }

    /// [`Self::save_view`], then `after` once the document is written: now, or on a later frame
    /// when the user had to choose where. `after` never runs when the save fails or is cancelled.
    pub(crate) fn save_then(&mut self, index: usize, target: SaveTarget, after: impl FnOnce(&mut Self) + Send + 'static) -> bool {
        // What's typed in a field is part of what's saved (#166).
        if let Some(id) = self.views.get(index).map(|v| v.id)
            && !self.commit_typing_in(id)
        {
            return false;
        }
        let Some(id) = self.views.get(index).map(|v| v.id) else { return false };
        let Some(doc) = self.session.get(id) else { return false };
        let (name, path) = (doc.name.clone(), doc.path.clone());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let doc_path = path.clone();
            let destination = match (target, path, &self.save_override) {
                (_, _, Some(p)) => p.clone(),
                // An email attachment's temporary copy is no place to keep work: ask, as for a new
                // document (`is_mail_temp_copy`).
                (SaveTarget::InPlace, Some(p), _) if !is_mail_temp_copy(&p) => p,
                _ => {
                    let name = if name.to_ascii_lowercase().ends_with(".pdf") { name } else { format!("{name}.pdf") };
                    let dialog = rfd::AsyncFileDialog::new().add_filter_any_case("PDF", &["pdf"]).set_file_name(name);
                    let dialog = in_document_folder(dialog, doc_path.as_deref());
                    // The bytes are taken once the user has chosen, so edits made meanwhile are saved too.
                    self.ask_one(crate::pickers::Ask::Save(dialog), None, move |app, dest| {
                        app.save_doc_to_then(id, &dest.to_string_lossy(), target, Box::new(after));
                    });
                    return false;
                }
            };
            self.save_doc_to_then(id, &destination, target, Box::new(after))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = path;
            if !self.flatten_fill_sign_for_save(id) {
                return false;
            }
            let bytes = match self.session.save_bytes(id) {
                Ok(b) => b,
                Err(e) => {
                    self.notify_fmt("Couldn't save {name}: {e}", &[("name", &name), ("e", &e.to_string())]);
                    return false;
                }
            };
            // The standalone browser picker completes asynchronously. Keep the document dirty
            // until the user chooses a destination and the bytes have actually been written.
            let Some(save) = &self.host_save else {
                let request_id = next_web_save_id();
                let generation = self.session.get(id).map(|doc| doc.edit_generation()).unwrap_or_default();
                queue_web_export(WebExport { name, bytes: bytes.clone(), request_id: Some(request_id) });
                self.pending_web_saves.push(PendingWebSave { request_id, doc_id: id, generation, bytes, after: Box::new(after) });
                return false;
            };
            let delivered = save(&name, &bytes, matches!(target, SaveTarget::As));
            match delivered {
                Ok(()) => {
                    let _ = self.session.mark_saved(id, bytes, None);
                    if let Some(doc) = self.session.get(id)
                        && let Some(view) = self.views.get_mut(index)
                    {
                        view.document_changed(&doc.info);
                    }
                    after(self);
                    true
                }
                Err(e) => {
                    self.notify_fmt("Couldn't save {name}: {e}", &[("name", &name), ("e", &e)]);
                    false
                }
            }
        }
    }

    /// [`Self::save_doc_to`], then `after`. When `dest` is the document's own file and it is
    /// another version on disk than the one the document came from (something else wrote it
    /// since the open or the last save, #2166), it asks first: the save waits in
    /// `pending_overwrite` behind [`Dialog::OverwriteChanged`](crate::Dialog::OverwriteChanged).
    /// Returns `true` if written now.
    #[cfg(not(target_arch = "wasm32"))]
    fn save_doc_to_then(&mut self, id: easypdf_engine::DocId, dest: &str, target: SaveTarget, after: Box<dyn FnOnce(&mut Self) + Send>) -> bool {
        let own = self.session.get(id).and_then(|d| d.path.as_deref()).is_some_and(|p| crate::same_file(p, dest));
        if own && self.watch.disk_differs(id, dest, self.session.get(id).map(|d| d.saved_bytes())) {
            self.pending_overwrite = Some(PendingOverwrite { id, dest: dest.to_string(), target, after });
            self.dialog = Some(crate::Dialog::OverwriteChanged);
            return false;
        }
        let saved = self.save_doc_to(id, dest, target);
        if saved {
            after(self);
        }
        saved
    }

    /// The dialog's Save anyway: write over the changed file.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn overwrite_changed_file(&mut self) {
        let Some(p) = self.pending_overwrite.take() else { return };
        if self.save_doc_to(p.id, &p.dest, p.target) {
            (p.after)(self);
        }
    }

    /// The dialog's Save as…: leave the file on disk as it is and ask where to put this version.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn save_changed_file_elsewhere(&mut self) {
        let Some(p) = self.pending_overwrite.take() else { return };
        if let Some(index) = self.views.iter().position(|v| v.id == p.id) {
            self.save_then(index, SaveTarget::As, p.after);
        }
    }

    /// Write document `id` to `dest` and make it the document's file. Returns `true` if written.
    #[cfg(not(target_arch = "wasm32"))]
    fn save_doc_to(&mut self, id: easypdf_engine::DocId, dest: &str, target: SaveTarget) -> bool {
        // Save As answers on a later frame: commit what was typed meanwhile, too (#166), even
        // if another tab is active by then.
        if !self.commit_typing_in(id) || !self.flatten_fill_sign_for_save(id) {
            return false;
        }
        let Some((name, full)) = self.session.get(id).map(|d| {
            // Save As to another file is a clean copy, without the earlier revisions that still
            // hold deleted pages or replaced text (#2165). A signed document stays incremental so
            // its signatures keep covering the file.
            let full = target == SaveTarget::As
                && !d.path.as_deref().is_some_and(|p| crate::same_file(p, dest))
                && d.read_only_reason.is_none()
                && !d.is_signed();
            (d.name.clone(), full)
        }) else {
            self.notify_tr("The document was closed before it could be saved.");
            return false;
        };
        let bytes = if full { self.session.save_full_bytes(id) } else { self.session.save_bytes(id) };
        let bytes = match bytes.and_then(|b| self.session.check_save(id, &b).map(|()| b)) {
            Ok(b) => b,
            Err(e) => {
                self.notify_fmt("Couldn't save {name}: {e}", &[("name", &name), ("e", &e.to_string())]);
                return false;
            }
        };
        if let Err(e) = write_atomically(dest, &bytes) {
            self.notify_fmt("Couldn't save {name}: {e}", &[("name", dest), ("e", &e.to_string())]);
            return false;
        }
        // Our own write is not a change on disk to reload (#431).
        self.watch.known_now(id, dest);
        match self.session.mark_saved(id, bytes, Some(dest.to_string())) {
            Ok(()) => {
                self.forget_recovery(id);
                if let Some(doc) = self.session.get(id)
                    && let Some(view) = self.views.iter_mut().find(|v| v.id == id)
                {
                    view.document_changed(&doc.info);
                    // Saving over a version that changed on disk keeps this one: nothing to ask.
                    view.disk_changed = false;
                }
                self.notify_fmt("Saved {name}", &[("name", &short_name(dest))]);
                #[cfg(target_os = "macos")]
                crate::macos_association::repair(std::path::Path::new(dest));
                true
            }
            Err(e) => {
                self.notify_fmt("Saved, but reopening failed: {e}", &[("e", &e.to_string())]);
                false
            }
        }
    }
}

/// Whether `path` is the temporary copy an email client opened an attachment from, so that Save
/// asks where to keep the document instead of writing to it.
///
/// Outlook opens an attachment from its own cache: `INetCache\Content.Outlook\<random>\` for
/// classic Outlook (`Temporary Internet Files\Content.Outlook\` on older Windows), and the
/// `Packages\Microsoft.OutlookForWindows_<id>` package folder for the new Outlook. The patterns
/// follow those layouts, so an ordinary folder that merely shares a name doesn't match. Saving there either fails,
/// because Outlook can still hold the file, or appears to work while the edits stay out of the
/// email and are deleted with the cache. Apple Mail opens one from `Mail Downloads/<UUID>/` in
/// its sandbox container, where other apps can't write, so saving there fails (#2751). Matched on
/// either separator and in any case, so it can be tested on every platform.
pub(crate) fn is_mail_temp_copy(path: &str) -> bool {
    let p = path.replace('\\', "/").to_ascii_lowercase();
    // Outlook's cache holds `<marker><random folder>/<file>`: exactly one folder, then the file.
    let in_outlook_cache = |marker: &str| {
        p.split_once(marker)
            .is_some_and(|(_, rest)| matches!(rest.split('/').collect::<Vec<_>>()[..], [dir, file] if !dir.is_empty() && !file.is_empty()))
    };
    in_outlook_cache("/inetcache/content.outlook/")
        || in_outlook_cache("/temporary internet files/content.outlook/")
        || p.contains("/packages/microsoft.outlookforwindows_")
        || p.contains("/library/containers/com.apple.mail/data/library/mail downloads/")
}

/// The folder a Save As style dialog should open in for a document stored at `path`: the
/// document's own folder, as Acrobat and Foxit do, instead of whichever folder the system picks.
/// `None` (keep the system's choice) for a new document, a path with no parent, or an email
/// attachment's temporary copy (`is_mail_temp_copy`), which is no place to keep work. Also right
/// for UNC paths on Windows (`\\server\share\dir\f.pdf` gives `\\server\share\dir`).
pub(crate) fn save_dialog_folder(path: Option<&str>) -> Option<std::path::PathBuf> {
    let path = path.filter(|p| !is_mail_temp_copy(p))?;
    let parent = std::path::Path::new(path).parent()?;
    (!parent.as_os_str().is_empty()).then(|| parent.to_path_buf())
}

/// `dialog` opened in the folder of the document stored at `path` ([`save_dialog_folder`]).
pub(crate) fn in_document_folder(dialog: rfd::AsyncFileDialog, path: Option<&str>) -> rfd::AsyncFileDialog {
    match save_dialog_folder(path) {
        Some(dir) => dialog.set_directory(dir),
        None => dialog,
    }
}

/// Write via a temporary file in the same directory and rename over the target, so a crash or
/// full disk never leaves a half-written PDF where the original was.
pub fn write_atomically(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    write_atomically_with(path, bytes, staging_suffixes())
}

/// [`write_atomically`], trying the staging names that `suffixes` give.
pub(super) fn write_atomically_with(path: &str, bytes: &[u8], suffixes: impl IntoIterator<Item = u64>) -> std::io::Result<()> {
    write_atomically_syncing(path, bytes, suffixes, std::fs::File::sync_all)
}

/// [`write_atomically_with`], flushing the staging file with `sync_all` (the standard library's
/// in the app, a failing one in tests). The writer itself is the platform crate's, shared with
/// the tools and the CLI (#2166); the desktop app keeps its staging-name form.
pub(super) fn write_atomically_syncing(
    path: &str,
    bytes: &[u8],
    suffixes: impl IntoIterator<Item = u64>,
    sync_all: impl FnOnce(&std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    easypdf_platform::staging::write_atomically_with(std::path::Path::new(path), bytes, StagingName::TagThenSuffix, suffixes, sync_all)
}

#[cfg(not(target_arch = "wasm32"))]
fn short_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

/// A standalone web save awaiting the browser's destination picker.
#[cfg(target_arch = "wasm32")]
pub(crate) struct PendingWebSave {
    request_id: u64,
    doc_id: easypdf_engine::DocId,
    generation: u64,
    bytes: std::sync::Arc<Vec<u8>>,
    after: Box<dyn FnOnce(&mut EasyPdfApp) + Send>,
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct WebExport {
    name: String,
    bytes: std::sync::Arc<Vec<u8>>,
    request_id: Option<u64>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(crate) enum WebSaveOutcome {
    Picked,
    DownloadedFallback,
    Cancelled,
}

#[cfg(target_arch = "wasm32")]
struct WebSaveEvent {
    request_id: Option<u64>,
    name: String,
    outcome: Result<WebSaveOutcome, String>,
}

#[cfg(target_arch = "wasm32")]
type WebOutputCallback = Box<dyn FnOnce(&mut EasyPdfApp, Result<WebSaveOutcome, String>) + Send>;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_SAVE_EVENTS: std::cell::RefCell<Vec<WebSaveEvent>> = const { std::cell::RefCell::new(Vec::new()) };
    static WEB_EXPORTS: std::cell::RefCell<Vec<WebExport>> = const { std::cell::RefCell::new(Vec::new()) };
    static WEB_SAVE_NEXT_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static WEB_SAVE_REPAINT: std::cell::RefCell<Option<egui::Context>> = const { std::cell::RefCell::new(None) };
    static WEB_OUTPUT_CALLBACKS: std::cell::RefCell<Vec<(u64, WebOutputCallback)>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(target_arch = "wasm32")]
fn next_web_save_id() -> u64 {
    WEB_SAVE_NEXT_ID.with(|next| {
        let value = next.get().wrapping_add(1);
        next.set(value);
        value
    })
}

#[cfg(target_arch = "wasm32")]
fn finish_web_save(event: WebSaveEvent) {
    WEB_SAVE_EVENTS.with(|events| events.borrow_mut().push(event));
    WEB_SAVE_REPAINT.with(|repaint| {
        if let Some(ctx) = repaint.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn queue_web_export(export: WebExport) {
    WEB_EXPORTS.with(|exports| exports.borrow_mut().push(export));
    WEB_SAVE_REPAINT.with(|repaint| {
        if let Some(ctx) = repaint.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

#[cfg(target_arch = "wasm32")]
impl EasyPdfApp {
    pub(crate) fn poll_web_saves(&mut self) {
        WEB_SAVE_REPAINT.with(|repaint| *repaint.borrow_mut() = self.ctx.clone());
        WEB_EXPORTS.with(|exports| self.pending_web_exports.extend(std::mem::take(&mut *exports.borrow_mut())));
        let events = WEB_SAVE_EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()));
        for event in events {
            let Some(request_id) = event.request_id else {
                match event.outcome {
                    Ok(WebSaveOutcome::Picked) => self.notify_fmt("Saved {name}", &[("name", &event.name)]),
                    Ok(WebSaveOutcome::DownloadedFallback) => self.notify_fmt("Download started: {name}", &[("name", &event.name)]),
                    Ok(WebSaveOutcome::Cancelled) => self.notify_tr("Cancelled"),
                    Err(e) => self.notify_fmt("Couldn't save {name}: {e}", &[("name", &event.name), ("e", &e)]),
                }
                continue;
            };
            let Some(position) = self.pending_web_saves.iter().position(|pending| pending.request_id == request_id) else {
                let callback = WEB_OUTPUT_CALLBACKS.with(|callbacks| {
                    let mut callbacks = callbacks.borrow_mut();
                    callbacks.iter().position(|(id, _)| *id == request_id).map(|index| callbacks.remove(index).1)
                });
                if let Some(callback) = callback {
                    callback(self, event.outcome);
                }
                continue;
            };
            let pending = self.pending_web_saves.remove(position);
            match event.outcome {
                Ok(WebSaveOutcome::Cancelled) => self.notify_tr("Cancelled"),
                Ok(WebSaveOutcome::DownloadedFallback) => {
                    self.notify_fmt("Download started: {name}; the document is still unsaved", &[("name", &event.name)]);
                }
                Err(e) => self.notify_fmt("Couldn't save {name}: {e}", &[("name", &event.name), ("e", &e)]),
                Ok(WebSaveOutcome::Picked) => {
                    let current = self.session.get(pending.doc_id).map(|doc| doc.edit_generation());
                    if current != Some(pending.generation) {
                        // The chosen file received the earlier bytes. Do not rebase a document
                        // edited while the picker was open, or discard its newer changes.
                        self.notify_tr("Saved a copy; the open document has newer changes.");
                        continue;
                    }
                    match self.session.mark_saved(pending.doc_id, pending.bytes, None) {
                        Ok(()) => {
                            if let Some(doc) = self.session.get(pending.doc_id)
                                && let Some(view) = self.views.iter_mut().find(|view| view.id == pending.doc_id)
                            {
                                view.document_changed(&doc.info);
                            }
                            self.notify_fmt("Saved {name}", &[("name", &event.name)]);
                            (pending.after)(self);
                        }
                        Err(e) => self.notify_fmt("Saved, but reopening failed: {e}", &[("e", &e.to_string())]),
                    }
                }
            }
        }
    }

    pub(crate) fn show_web_export_prompt(&mut self, ctx: &egui::Context) {
        let Some(export) = self.pending_web_exports.front() else { return };
        let mut action = None;
        egui::Window::new(tl!("Save as…"))
            .id(egui::Id::new("web-export-destination"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&export.name);
                ui.horizontal(|ui| {
                    if ui.button(tl!("Save as…")).clicked() {
                        action = Some("choose");
                    }
                    if ui.button(tl!("Download")).clicked() {
                        action = Some("download");
                    }
                    if ui.button(tl!("Cancel")).clicked() {
                        action = Some("cancel");
                    }
                });
            });
        let Some(action) = action else { return };
        let Some(export) = self.pending_web_exports.pop_front() else { return };
        match action {
            "choose" => {
                if let Err(e) = offer_destination(&export.name, export.bytes, export.request_id) {
                    finish_web_save(WebSaveEvent { request_id: export.request_id, name: export.name, outcome: Err(e) });
                }
            }
            "download" => {
                let outcome = download_anchor(&export.name, &export.bytes).map(|()| WebSaveOutcome::DownloadedFallback);
                finish_web_save(WebSaveEvent { request_id: export.request_id, name: export.name, outcome });
            }
            _ => finish_web_save(WebSaveEvent { request_id: export.request_id, name: export.name, outcome: Ok(WebSaveOutcome::Cancelled) }),
        }
    }
}

/// Offer exported bytes to the browser's destination picker. Browsers without the File System
/// Access API use their ordinary named download; the browser then chooses the download folder.
#[cfg(target_arch = "wasm32")]
pub(crate) fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
    queue_web_export(WebExport { name: name.to_string(), bytes: std::sync::Arc::new(bytes.to_vec()), request_id: None });
    Ok(())
}

/// Queue an output and run `after` only when the browser reports its result. A named download
/// confirms that it was started, not that the browser wrote the file.
#[cfg(target_arch = "wasm32")]
pub(crate) fn download_then(name: &str, bytes: &[u8], after: impl FnOnce(&mut EasyPdfApp, Result<WebSaveOutcome, String>) + Send + 'static) {
    let request_id = next_web_save_id();
    WEB_OUTPUT_CALLBACKS.with(|callbacks| callbacks.borrow_mut().push((request_id, Box::new(after))));
    queue_web_export(WebExport { name: name.to_string(), bytes: std::sync::Arc::new(bytes.to_vec()), request_id: Some(request_id) });
}

#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Clone, Copy)]
enum BatchWriteResult {
    Picked,
    DownloadedFallback,
    Cancelled,
    Failed,
}

#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Default)]
struct WebBatchState {
    queued: usize,
    resolved: usize,
    picked: usize,
    downloaded: usize,
    cancelled: usize,
    failed: usize,
    finished: bool,
    reported: bool,
    detail: Option<String>,
    first_error: Option<String>,
}

#[cfg(any(test, target_arch = "wasm32"))]
impl WebBatchState {
    fn record(&mut self, result: BatchWriteResult) {
        self.resolved += 1;
        match result {
            BatchWriteResult::Picked => self.picked += 1,
            BatchWriteResult::DownloadedFallback => self.downloaded += 1,
            BatchWriteResult::Cancelled => self.cancelled += 1,
            BatchWriteResult::Failed => self.failed += 1,
        }
    }

    fn ready(&self) -> bool {
        self.finished && self.resolved == self.queued && !self.reported
    }
}

/// Collect the outcome of every destination prompt in one browser batch. Processing a file and
/// writing it are separate events; the summary is shown only after every prompt resolves.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default)]
pub(crate) struct WebBatchWrites {
    state: std::sync::Arc<std::sync::Mutex<WebBatchState>>,
}

#[cfg(target_arch = "wasm32")]
impl WebBatchWrites {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn queued(&self) -> usize {
        self.state.lock().map(|state| state.queued).unwrap_or(0)
    }

    pub(crate) fn queue(&self, name: &str, bytes: &[u8]) {
        if let Ok(mut state) = self.state.lock() {
            state.queued += 1;
        }
        let batch = self.clone();
        let filename = name.to_string();
        download_then(name, bytes, move |app, outcome| {
            let error = outcome.as_ref().err().map(|error| format!("{filename}: {error}"));
            let result = match outcome {
                Ok(WebSaveOutcome::Picked) => BatchWriteResult::Picked,
                Ok(WebSaveOutcome::DownloadedFallback) => BatchWriteResult::DownloadedFallback,
                Ok(WebSaveOutcome::Cancelled) => BatchWriteResult::Cancelled,
                Err(_) => BatchWriteResult::Failed,
            };
            let summary = batch.state.lock().ok().and_then(|mut state| {
                state.record(result);
                if state.first_error.is_none() {
                    state.first_error = error;
                }
                web_batch_summary(&mut state)
            });
            if let Some(summary) = summary {
                app.notify(summary);
            }
        });
    }

    pub(crate) fn finish(&self, app: &mut EasyPdfApp, detail: Option<String>) {
        let summary = self.state.lock().ok().and_then(|mut state| {
            state.finished = true;
            state.detail = detail;
            web_batch_summary(&mut state)
        });
        if let Some(summary) = summary {
            app.notify(summary);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn web_batch_summary(state: &mut WebBatchState) -> Option<String> {
    if !state.ready() {
        return None;
    }
    state.reported = true;
    let mut message = if state.queued == 0 {
        tl!("No files were produced").to_string()
    } else {
        crate::i18n::fmt(
            tl!("Batch outputs: {saved} saved, {downloads} downloads started, {cancelled} cancelled, {failed} failed"),
            &[
                ("saved", &state.picked.to_string()),
                ("downloads", &state.downloaded.to_string()),
                ("cancelled", &state.cancelled.to_string()),
                ("failed", &state.failed.to_string()),
            ],
        )
    };
    if let Some(detail) = state.detail.take() {
        message.push_str("; ");
        message.push_str(&detail);
    }
    if let Some(error) = state.first_error.take() {
        message.push_str(&crate::i18n::fmt(tl!("; first write error: {error}"), &[("error", &error)]));
    }
    Some(message)
}

#[cfg(test)]
mod batch_state_tests {
    use super::{BatchWriteResult, WebBatchState};

    #[test]
    fn waits_for_every_destination_and_counts_mixed_outcomes_once() {
        let mut state = WebBatchState { queued: 3, finished: true, ..Default::default() };
        state.record(BatchWriteResult::Picked);
        assert!(!state.ready());
        state.record(BatchWriteResult::Cancelled);
        assert!(!state.ready());
        state.record(BatchWriteResult::DownloadedFallback);
        assert!(state.ready());
        assert_eq!((state.picked, state.downloaded, state.cancelled, state.failed), (1, 1, 1, 0));
        state.reported = true;
        assert!(!state.ready());
    }

    #[test]
    fn processing_must_finish_before_a_batch_can_report() {
        let mut state = WebBatchState { queued: 1, ..Default::default() };
        state.record(BatchWriteResult::Failed);
        assert!(!state.ready());
        state.finished = true;
        assert!(state.ready());
    }

    #[test]
    fn keeps_batch_detail_and_first_write_error_until_the_summary() {
        let state = WebBatchState {
            detail: Some("2 pages processed".to_string()),
            first_error: Some("page-2.png: permission denied".to_string()),
            ..Default::default()
        };

        assert_eq!(state.detail.as_deref(), Some("2 pages processed"));
        assert_eq!(state.first_error.as_deref(), Some("page-2.png: permission denied"));
    }
}

#[cfg(target_arch = "wasm32")]
fn offer_destination(name: &str, bytes: std::sync::Arc<Vec<u8>>, request_id: Option<u64>) -> Result<(), String> {
    use wasm_bindgen::{JsCast, JsValue};
    let window = web_sys::window().ok_or("no window")?;
    let picker = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("showSaveFilePicker")).map_err(|e| format!("{e:?}"))?;
    if !picker.is_function() {
        download_anchor(name, &bytes)?;
        finish_web_save(WebSaveEvent { request_id, name: name.to_string(), outcome: Ok(WebSaveOutcome::DownloadedFallback) });
        return Ok(());
    }
    let picker: js_sys::Function = picker.dyn_into().map_err(|_| "save picker is not callable")?;
    let options = js_sys::Object::new();
    js_sys::Reflect::set(&options, &JsValue::from_str("suggestedName"), &JsValue::from_str(name)).map_err(|e| format!("{e:?}"))?;
    let selection = match picker.call1(window.as_ref(), &options) {
        Ok(selection) => selection,
        Err(e) if js_error_name(&e).as_deref() == Some("SecurityError") => {
            download_anchor(name, &bytes)?;
            finish_web_save(WebSaveEvent { request_id, name: name.to_string(), outcome: Ok(WebSaveOutcome::DownloadedFallback) });
            return Ok(());
        }
        Err(e) => return Err(format!("{e:?}")),
    };
    let selection: js_sys::Promise = selection.dyn_into().map_err(|_| "save picker returned no promise")?;
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let result = write_to_browser_file(selection, &bytes).await;
        let outcome = match result {
            Ok(()) => Ok(WebSaveOutcome::Picked),
            Err(e) if js_error_name(&e).as_deref() == Some("AbortError") => Ok(WebSaveOutcome::Cancelled),
            Err(e) if js_error_name(&e).as_deref() == Some("SecurityError") => {
                download_anchor(&name, &bytes).map(|()| WebSaveOutcome::DownloadedFallback)
            }
            Err(e) => Err(format!("{e:?}")),
        };
        finish_web_save(WebSaveEvent { request_id, name, outcome });
    });
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn js_error_name(error: &wasm_bindgen::JsValue) -> Option<String> {
    js_sys::Reflect::get(error, &wasm_bindgen::JsValue::from_str("name")).ok()?.as_string()
}

#[cfg(target_arch = "wasm32")]
async fn write_to_browser_file(selection: js_sys::Promise, bytes: &[u8]) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    let handle = JsFuture::from(selection).await?;
    let create = js_sys::Reflect::get(&handle, &JsValue::from_str("createWritable"))?.dyn_into::<js_sys::Function>()?;
    let writable = JsFuture::from(create.call0(&handle)?.dyn_into::<js_sys::Promise>()?).await?;
    let write = js_sys::Reflect::get(&writable, &JsValue::from_str("write"))?.dyn_into::<js_sys::Function>()?;
    let data = js_sys::Uint8Array::from(bytes);
    JsFuture::from(write.call1(&writable, &data)?.dyn_into::<js_sys::Promise>()?).await?;
    let close = js_sys::Reflect::get(&writable, &JsValue::from_str("close"))?.dyn_into::<js_sys::Function>()?;
    JsFuture::from(close.call0(&writable)?.dyn_into::<js_sys::Promise>()?).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn download_anchor(name: &str, bytes: &[u8]) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let err = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array);
    let extension = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    let mime = match extension.as_str() {
        "pdf" => "application/pdf",
        "csv" => "text/csv",
        "xfdf" => "application/vnd.adobe.xfdf",
        "rtf" => "application/rtf",
        "html" | "htm" => "text/html",
        "txt" => "text/plain",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    };
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let document = web_sys::window().and_then(|window| window.document()).ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(err)?.dyn_into().map_err(|_| "anchor")?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}
