//! An upstream update feed must not be reachable in the easyPDF staging build.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use easypdf_ui_egui::EasyPdfApp;
use easypdf_ui_egui::updates::UpdateSource;
use egui_kittest::Harness;

#[test]
fn unsigned_update_channel_stays_off_even_when_requested() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let source: UpdateSource = Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    });
    let mut harness = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_cc| {
        let mut app = EasyPdfApp::new();
        app.set_update_source(source);
        app.set_startup_update_check(true);
        app
    });
    harness.state_mut().check_for_updates();
    assert!(!harness.state_mut().execute("help.check_updates"));
    harness.run_steps(4);
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no unsigned release request may start");
}
