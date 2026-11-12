//! Keep easyPDF's own cover-tool label translated across the shared web/desktop UI.
//! Existing translations come from easyPDF's earlier local catalogs; the remaining
//! locale labels are original easyPDF wording, pending native-speaker review.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::{atomic_write, read_bounded, sha256};

const BUILD_SCRIPT_SHA256: &str = "C5B27F7A90F3E3A297C94881A88ACACCEA50E04C03E7D6C8C3ED1836EFDC5139";
// The independently prepared replay staging differs only in rustfmt layout.
const REPLAY_BUILD_SCRIPT_SHA256: &str = "7496510E5C501E75C1CEB71124B3493109D87E20B76C4F1B745C361A10A0C800";
const COVER_TRANSLATIONS: [(&str, &str); 44] = [
    ("ar", "تطبيق الأغلفة"),
    ("bg", "Прилагане на корици"),
    ("bn", "প্রচ্ছদ প্রয়োগ করুন"),
    ("bs", "Primijeni naslovnice"),
    ("ca", "Aplica les portades"),
    ("ckb", "بەرگەکان جێبەجێ بکە"),
    ("cs", "Použít obálky"),
    ("da", "Anvend forsider"),
    ("de", "Umschläge anwenden"),
    ("dv", "ކަވަރ ޞަފްޙާތައް ހިންގާ"),
    ("el", "Εφαρμογή εξωφύλλων"),
    ("es", "Aplicar portadas"),
    ("fa", "اعمال جلدها"),
    ("fi", "Lisää kannet"),
    ("fr", "Appliquer les couvertures"),
    ("he", "החלת עמודי שער"),
    ("hi", "कवर लागू करें"),
    ("hu", "Borítók alkalmazása"),
    ("id", "Terapkan sampul"),
    ("it", "Applica cover"),
    ("ja", "表紙を適用"),
    ("ka", "თავფურცლების გამოყენება"),
    ("kk", "Мұқабаларды қолдану"),
    ("ko", "표지 적용"),
    ("lo", "ນຳໃຊ້ໜ້າປົກ"),
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
    ("ta", "அட்டைப் பக்கங்களைப் பயன்படுத்து"),
    ("te", "కవర్‌లను వర్తింపజేయి"),
    ("th", "ใช้หน้าปก"),
    ("tr", "Kapakları uygula"),
    ("uk", "Застосувати обкладинки"),
    ("ur", "سرورق لاگو کریں"),
    ("vi", "Áp dụng bìa"),
    ("zh-hans", "应用封面"),
    ("zh-hant", "套用封面"),
];

const IT_COVER_DIALOG_SOURCE: &str = r#"// easyPDF-specific dialog text absent from the pinned upstream catalogs.
const IT_COVER_DIALOG_TRANSLATIONS: &[(&str, &str)] = &[
    ("Insert every front-cover page before the document and every back-cover page after it.", "Inserisci le pagine della cover frontale prima del documento e quelle della cover posteriore dopo."),
    ("Choose a cover kit", "Scegli un kit cover"),
    ("Front cover", "Cover frontale"),
    ("Back cover", "Cover posteriore"),
    ("Not selected", "Non selezionata"),
    ("Choose PDF…", "Scegli PDF…"),
    ("Save these covers as a kit", "Salva queste cover come kit"),
    ("Cover kit name", "Nome del kit cover"),
    ("Use as default cover kit", "Usa come kit cover predefinito"),
    ("The covers are applied as one undoable change.", "Le cover vengono applicate con un'unica modifica annullabile."),
];"#;

pub(super) fn apply(root: &Path, stage: &Path, backup_label: &str) -> Result<()> {
    let relative = "crates/ui-egui/build.rs";
    let original = read_bounded(&stage.join(relative))?;
    let original_hash = sha256(&original);
    if !original_hash.eq_ignore_ascii_case(BUILD_SCRIPT_SHA256) && !original_hash.eq_ignore_ascii_case(REPLAY_BUILD_SCRIPT_SHA256) {
        bail!("review required: staged UI catalog builder changed");
    }
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-cover-translations-before-apply"));
    if backup.exists() {
        bail!("cover translation backup already exists: {}", backup.display());
    }
    let source = String::from_utf8(original.clone()).context("decode UI catalog builder")?;
    let table_anchor = "const APP_CATALOG: &str = \"pdfcraft.tsv\";";
    if source.matches(table_anchor).count() != 1 {
        bail!("review required: UI catalog builder table anchor changed");
    }
    let mut rows = String::new();
    for (locale, translation) in COVER_TRANSLATIONS {
        rows.push_str(&format!("    ({locale:?}, {translation:?}),\n"));
    }
    let table = format!(
        "{table_anchor}\n\n// easyPDF-owned tool label, shared by the web and desktop catalogs.\nconst EASY_PDF_COVER_TRANSLATIONS: &[(&str, &str)] = &[\n{rows}];\n\n{IT_COVER_DIALOG_SOURCE}"
    );
    let mut updated = source.replacen(table_anchor, &table, 1);
    let old = "let branded = source.replace(\"PdfCraft\", \"easyPDF\").replace(\"pdfcraft\", \"easypdf\").replace(\"ArtCraft\", \"easyPDF\");";
    if updated.matches(old).count() != 1 {
        bail!("review required: UI catalog builder branding anchor changed");
    }
    let new = r#"let mut branded = source.replace("PdfCraft", "easyPDF").replace("pdfcraft", "easypdf").replace("ArtCraft", "easyPDF");
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
        }"#;
    updated = updated.replacen(old, new, 1);
    // The replay checkout retained an older comment spelling; keep both outputs byte-identical.
    updated = updated.replace(
        "translated labels and English lookup keys match easyPDF's UI source.",
        "translated labels and their English lookup keys match easyPDF's UI source.",
    );
    let mut formatter = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start rustfmt for the UI catalog builder")?;
    {
        let mut stdin = formatter.stdin.take().context("open rustfmt input")?;
        stdin.write_all(updated.as_bytes()).context("send UI catalog builder to rustfmt")?;
    }
    let formatted = formatter.wait_with_output().context("format UI catalog builder")?;
    if !formatted.status.success() {
        bail!("rustfmt rejected UI catalog builder: {}", String::from_utf8_lossy(&formatted.stderr));
    }
    atomic_write(&backup.join(relative), &original)?;
    atomic_write(&stage.join(relative), &formatted.stdout)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::COVER_TRANSLATIONS;

    #[test]
    fn every_embedded_non_english_catalog_has_one_cover_label() {
        let codes: BTreeSet<&str> = COVER_TRANSLATIONS.iter().map(|(code, _)| *code).collect();
        assert_eq!(codes.len(), 44);
        assert!(COVER_TRANSLATIONS.iter().all(|(_, label)| !label.trim().is_empty() && !label.ends_with('…')));
    }
}
