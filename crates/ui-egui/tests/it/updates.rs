//! The easyPDF update flow uses its replaceable, official release source.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use easypdf_ui_egui::EasyPdfApp;
use easypdf_ui_egui::updates::UpdateSource;
use egui_kittest::Harness;

#[test]
fn requested_update_check_uses_the_easy_pdf_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let source: UpdateSource = Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    });
    let mut harness = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_cc| {
        let mut app = EasyPdfApp::new();
        app.set_update_source(source);
        app.set_startup_update_check(false);
        app
    });
    harness.state_mut().check_for_updates();
    harness.run_steps(8);
    assert_eq!(calls.load(Ordering::SeqCst), 1, "the official release source is checked once");
}
