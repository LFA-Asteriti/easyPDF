//! Embed the optional interface catalogs; contributor credits are not part of easyPDF.
use std::fmt::Write as _;

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default());
    let out = out_dir.join("craft_i18n.rs");
    if let Err(error) = std::fs::write(&out, craft_i18n(&out_dir)) {
        println!("cargo::error=writing {}: {error}", out.display());
    }
}

/// The catalog this app reads in each locale: `locales/<locale>/pdfcraft.tsv`.
const APP_CATALOG: &str = "pdfcraft.tsv";

// easyPDF-owned tool label, shared by the web and desktop catalogs.
const EASY_PDF_COVER_TRANSLATIONS: &[(&str, &str)] = &[
    ("ar", "تطبيق الأغلفة"),
    ("bg", "Прилагане на корици"),
    ("bn", "প\u{9cd}রচ\u{9cd}ছদ প\u{9cd}রয\u{9bc}োগ কর\u{9c1}ন"),
    ("bs", "Primijeni naslovnice"),
    ("ca", "Aplica les portades"),
    ("ckb", "بەرگەکان جێبەجێ بکە"),
    ("cs", "Použít obálky"),
    ("da", "Anvend forsider"),
    ("de", "Umschläge anwenden"),
    ("dv", "ކ\u{7a6}ވ\u{7a6}ރ ޞ\u{7a6}ފ\u{7b0}ޙ\u{7a7}ތ\u{7a6}އ\u{7b0} ހ\u{7a8}ނ\u{7b0}ގ\u{7a7}"),
    ("el", "Εφαρμογή εξωφύλλων"),
    ("es", "Aplicar portadas"),
    ("fa", "اعمال جلدها"),
    ("fi", "Lisää kannet"),
    ("fr", "Appliquer les couvertures"),
    ("he", "החלת עמודי שער"),
    ("hi", "कवर लाग\u{942} कर\u{947}\u{902}"),
    ("hu", "Borítók alkalmazása"),
    ("id", "Terapkan sampul"),
    ("it", "Applica cover"),
    ("ja", "表紙を適用"),
    ("ka", "თავფურცლების გამოყენება"),
    ("kk", "Мұқабаларды қолдану"),
    ("ko", "표지 적용"),
    ("lo", "ນຳໃຊ\u{ec9}ໜ\u{ec9}າປ\u{ebb}ກ"),
    ("lt", "Taikyti viršelius"),
    ("nb", "Bruk omslag"),
    ("nl", "Voorbladen toepassen"),
    ("pl", "Zastosuj okładki"),
    ("pt-br", "Aplicar capas"),
    ("ro", "Aplică coperțile"),
    ("ru", "Применить обложки"),
    ("sl", "Uporabi naslovnice"),
    ("sq", "Apliko kopertinat"),
    ("sv", "Använd omslagssidor"),
    ("ta", "அட\u{bcd}டைப\u{bcd} பக\u{bcd}கங\u{bcd}களைப\u{bcd} பயன\u{bcd}படுத\u{bcd}து"),
    ("te", "కవర\u{c4d}\u{200c}లను వర\u{c4d}త\u{c3f}ంపజ\u{c47}య\u{c3f}"),
    ("th", "ใช\u{e49}หน\u{e49}าปก"),
    ("tr", "Kapakları uygula"),
    ("uk", "Застосувати обкладинки"),
    ("ur", "سرورق لاگو کریں"),
    ("vi", "Áp dụng bìa"),
    ("zh-hans", "应用封面"),
    ("zh-hant", "套用封面"),
];

// easyPDF-specific dialog text absent from the pinned upstream catalogs.
const IT_COVER_DIALOG_TRANSLATIONS: &[(&str, &str)] = &[
    (
        "Insert every front-cover page before the document and every back-cover page after it.",
        "Inserisci le pagine della cover frontale prima del documento e quelle della cover posteriore dopo.",
    ),
    ("Choose a cover kit", "Scegli un kit cover"),
    ("Front cover", "Cover frontale"),
    ("Back cover", "Cover posteriore"),
    ("Not selected", "Non selezionata"),
    ("Choose PDF…", "Scegli PDF…"),
    ("Save these covers as a kit", "Salva queste cover come kit"),
    ("Cover kit name", "Nome del kit cover"),
    ("Use as default cover kit", "Usa come kit cover predefinito"),
    ("The covers are applied as one undoable change.", "Le cover vengono applicate con un'unica modifica annullabile."),
];

/// `CRAFT_I18N` from `CRAFT_I18N_DIR`: the `Embedded` list the craft-i18n crate reads.
fn craft_i18n(out_dir: &std::path::Path) -> String {
    println!("cargo::rerun-if-env-changed=CRAFT_I18N_DIR");
    println!("cargo::rerun-if-env-changed=CRAFT_I18N_REQUIRED");
    let required = std::env::var("CRAFT_I18N_REQUIRED").is_ok_and(|v| v.trim() == "1");
    let entries = match std::env::var_os("CRAFT_I18N_DIR").filter(|d| !d.is_empty()) {
        // Not canonicalized: on Windows that gives `\\?\C:\…`, which `include_str!` can't read.
        Some(dir) => match locales(&std::path::PathBuf::from(dir).join("locales"), out_dir) {
            Ok(entries) => entries,
            Err(e) => {
                let level = if required { "error" } else { "warning" };
                println!("cargo::{level}=CRAFT_I18N_DIR: {e}; the interface is English only");
                Vec::new()
            }
        },
        None => {
            if required {
                println!("cargo::error=CRAFT_I18N_REQUIRED=1 but CRAFT_I18N_DIR is unset: a release must embed the translations");
            }
            Vec::new()
        }
    };
    let mut o = String::from("/// (locale, shared catalog, easyPDF's catalog), one entry per locale; `\"\"` for a missing file.\n");
    o.push_str("pub static CRAFT_I18N: &craft_i18n::Embedded = &[\n");
    for (code, shared, app) in entries {
        // `{:?}` escapes the path (Windows backslashes) as a Rust string literal.
        let shared = shared.map_or_else(|| "\"\"".to_string(), |p| format!("include_str!({p:?})"));
        let _ = writeln!(o, "    ({code:?}, {shared}, include_str!({app:?})),");
    }
    o.push_str("];\n");
    o
}

/// One embedded locale: its code, its `common.tsv` if it has one, and its easyPDF catalog.
type Locale = (String, Option<std::path::PathBuf>, std::path::PathBuf);

/// Every locale under `dir` with easyPDF's catalog, by code. An error when `dir` isn't a
/// craft-i18n `locales/` directory.
fn locales(dir: &std::path::Path, out_dir: &std::path::Path) -> Result<Vec<Locale>, String> {
    // Build scripts run in the crate's directory and `include_str!` resolves against the
    // generated file, so a relative path would mean two different places.
    if !dir.is_absolute() {
        return Err(format!("{} is not an absolute path", dir.display()));
    }
    // Before read_dir, so a checkout that appears later reruns the script.
    println!("cargo::rerun-if-changed={}", dir.display());
    let list = std::fs::read_dir(dir).map_err(|e| format!("{} is not a craft-i18n checkout ({e})", dir.display()))?;
    let mut out = Vec::new();
    for entry in list.flatten() {
        let path = entry.path();
        let Some(code) = path.file_name().and_then(|n| n.to_str()).map(str::to_string) else { continue };
        let app = path.join(APP_CATALOG);
        if !app.is_file() {
            continue;
        }
        println!("cargo::rerun-if-changed={}", app.display());
        let shared = Some(path.join("common.tsv")).filter(|p| p.is_file());
        if let Some(p) = &shared {
            println!("cargo::rerun-if-changed={}", p.display());
        }
        // Keep the separately licensed source checkout unchanged. Embed a branded copy so
        // translated labels and their English lookup keys match easyPDF's UI source.
        let size = std::fs::metadata(&app).map_err(|e| format!("{}: {e}", app.display()))?.len();
        if size > 8 * 1024 * 1024 {
            return Err(format!("{} exceeds the 8 MiB catalog limit", app.display()));
        }
        let source = std::fs::read_to_string(&app).map_err(|e| format!("{}: {e}", app.display()))?;
        let mut branded = source.replace("PdfCraft", "easyPDF").replace("pdfcraft", "easypdf").replace("ArtCraft", "easyPDF");
        if let Some((_, translation)) = EASY_PDF_COVER_TRANSLATIONS.iter().find(|(locale, _)| *locale == code.as_str()) {
            if branded.contains("\tApply covers\t") || branded.contains("\tApply covers…\t") {
                return Err(format!("{code}: upstream now owns Apply covers translations; review before merging"));
            }
            branded.push_str(&format!("\n\tApply covers\t{translation}\n\tApply covers…\t{translation}…\n"));
        }
        if code == "it" {
            branded.push_str("\n\teasyPDF processes local files on your device. The protected web edition uses your authorized account.\teasyPDF elabora i file locali sul dispositivo. La versione web protetta utilizza il tuo account autorizzato.\n");
            for (source, translation) in IT_COVER_DIALOG_TRANSLATIONS {
                branded.push_str(&format!("\t{source}\t{translation}\n"));
            }
        }
        let branded_path = out_dir.join(format!("easypdf-{code}.tsv"));
        std::fs::write(&branded_path, branded).map_err(|e| format!("{}: {e}", branded_path.display()))?;
        out.push((code, shared, branded_path));
    }
    if out.is_empty() {
        return Err(format!("{} has no locale with a {APP_CATALOG}", dir.display()));
    }
    out.sort();
    Ok(out)
}
