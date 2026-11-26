//! Safe easyPDF update checks.
//!
//! Desktop builds look only at releases published in the official easyPDF repository. Because
//! beta packages are currently unsigned, the app never replaces its own executable: it opens the
//! exact package for the current platform and lets the operating system handle the installation.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, TryRecvError, channel};

#[cfg(not(target_arch = "wasm32"))]
use craft_update::{Http, Platform, Release, check_for_update, pick_asset};

use crate::EasyPdfApp;

/// Where every easyPDF release is listed.
pub const RELEASES_PAGE: &str = "https://github.com/LFA-Asteriti/easyPDF/releases";

/// Stands in for the network in tests.
#[cfg(not(target_arch = "wasm32"))]
pub type UpdateSource = Arc<dyn Fn() -> Result<Option<Release>, String> + Send + Sync>;

#[cfg(not(target_arch = "wasm32"))]
type Checked = Result<Option<Release>, String>;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Failed(String),
}

pub(crate) struct Updates {
    /// The Updates dialog is showing.
    pub(crate) open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    startup_check: bool,
    #[cfg(not(target_arch = "wasm32"))]
    check_requested: bool,
    #[cfg(not(target_arch = "wasm32"))]
    requested: bool,
    #[cfg(not(target_arch = "wasm32"))]
    source: Option<UpdateSource>,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: Option<Receiver<Checked>>,
    #[cfg(not(target_arch = "wasm32"))]
    status: UpdateStatus,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            open: false,
            #[cfg(not(target_arch = "wasm32"))]
            startup_check: false,
            #[cfg(not(target_arch = "wasm32"))]
            check_requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            source: None,
            #[cfg(not(target_arch = "wasm32"))]
            receiver: None,
            #[cfg(not(target_arch = "wasm32"))]
            status: UpdateStatus::Idle,
        }
    }
}

impl EasyPdfApp {
    /// Enables the single check performed after the first desktop frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates.startup_check = on;
    }

    /// Replaces the network with `source` (tests).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_update_source(&mut self, source: UpdateSource) {
        self.updates.source = Some(source);
    }

    /// Checks the official release channel and shows the outcome.
    pub fn check_for_updates(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.updates.open = true;
            self.updates.check_requested = true;
        }
        #[cfg(target_arch = "wasm32")]
        self.open_url(RELEASES_PAGE);
    }

    /// Starts checks and picks up their answers (each frame).
    pub(crate) fn poll_updates(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(ctx) = self.ctx.clone() else { return };
            let startup = std::mem::take(&mut self.updates.startup_check) && self.update_check_at_start;
            let requested = std::mem::take(&mut self.updates.check_requested);
            if self.updates.receiver.is_none() && (startup || requested) {
                self.updates.requested = requested;
                self.updates.status = UpdateStatus::Checking;
                let source = self.updates.source.clone().unwrap_or_else(default_source);
                let (sender, receiver) = channel();
                self.updates.receiver = Some(receiver);
                let repaint = ctx.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(source());
                    repaint.request_repaint();
                });
            }

            let received = self.updates.receiver.as_ref().and_then(|receiver| match receiver.try_recv() {
                Ok(answer) => Some(answer),
                Err(TryRecvError::Disconnected) => Some(Err("The update check stopped unexpectedly.".to_string())),
                Err(TryRecvError::Empty) => None,
            });
            if let Some(answer) = received {
                self.updates.receiver = None;
                self.updates.status = match answer {
                    Ok(Some(release)) => UpdateStatus::Available(release),
                    Ok(None) => UpdateStatus::UpToDate,
                    Err(error) => UpdateStatus::Failed(error),
                };
                if self.updates.requested || matches!(self.updates.status, UpdateStatus::Available(_)) {
                    self.updates.open = true;
                }
                self.updates.requested = false;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn default_source() -> UpdateSource {
    Arc::new(|| {
        let http = Http::new(&format!("easyPDF/{}", env!("CARGO_PKG_VERSION")))?;
        check_for_update(&crate::UPDATE_APP, &http, env!("CARGO_PKG_VERSION"))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn download_url(release: &Release) -> String {
    Platform::current()
        .and_then(|platform| pick_asset(release, &["easypdf"], platform, platform.default_kinds()).map(|(asset, _)| asset.url.clone()))
        .unwrap_or_else(|| release.html_url.clone())
}

/// Updates dialog. Web deployments are updated by their server and do not show it.
pub(crate) fn dialog(app: &mut EasyPdfApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !app.updates.open {
            return;
        }
        let status = app.updates.status.clone();
        let mut open = app.updates.open;
        let mut close_clicked = false;
        let mut check_again = false;
        let mut url = None;
        egui::Window::new(tl!("Updates")).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(format!("{}: {}", tl!("Version"), env!("CARGO_PKG_VERSION")));
            ui.add_space(8.0);
            match status {
                UpdateStatus::Idle | UpdateStatus::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tl!("Checking for updates…"));
                    });
                }
                UpdateStatus::UpToDate => {
                    ui.label(tl!("easyPDF is up to date."));
                }
                UpdateStatus::Available(ref release) => {
                    ui.label(tl!("An easyPDF update is available."));
                    ui.strong(format!("easyPDF {}", release.version_label()));
                    ui.label(tl!("Download the package and run it to update easyPDF."));
                    ui.add_space(8.0);
                    if ui.button(tl!("Download update")).clicked() {
                        url = Some(download_url(release));
                    }
                }
                UpdateStatus::Failed(ref error) => {
                    ui.label(tl!("The update check failed."));
                    ui.label(error);
                    ui.add_space(8.0);
                    if ui.button(tl!("Try again")).clicked() {
                        check_again = true;
                    }
                }
            }
            ui.add_space(8.0);
            if ui.button(tl!("Close")).clicked() {
                close_clicked = true;
            }
        });
        app.updates.open = open && !close_clicked;
        if check_again {
            app.updates.check_requested = true;
            app.updates.requested = true;
            app.updates.status = UpdateStatus::Checking;
        }
        if let Some(url) = url {
            app.open_url(&url);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}
