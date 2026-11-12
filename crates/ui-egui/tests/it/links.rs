//! Help links follow the installed portal and never display upstream contributor UI.

use easypdf_engine::links;
use easypdf_ui_egui::{Dialog, EasyPdfApp};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness(setup: impl FnOnce(&mut EasyPdfApp) + 'static) -> Harness<'static, EasyPdfApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = EasyPdfApp::new();
        app.set_option("language", "en").unwrap();
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

#[test]
fn help_commands_follow_configured_portal() {
    for link in links::LINKS {
        let mut h = harness(|app| app.portal_url = "https://pdf.example.test".to_owned());
        assert!(h.state_mut().execute(link.command));
        assert_eq!(h.state().last_opened_url.as_deref(), Some(links::portal_link(Some("https://pdf.example.test"), link.url).as_str()));
    }
}

#[test]
fn about_shows_easy_pdf_without_contributors() {
    let h = harness(|app| app.dialog = Some(Dialog::About));
    assert!(h.query_all_by_label("easyPDF by Massimo Bertolucci").next().is_some());
    assert_eq!(h.query_all_by_label("Contributors").count(), 0);
}

