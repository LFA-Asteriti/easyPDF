//! Embeds the audited easyPDF font bundle stored in `assets/easypdf-fonts`.
//! The build is self-contained: it never downloads fonts and has no font-repository input.
//! Native builds embed every approved manifest face. Web builds keep only the UI faces needed
//! by the browser build so the WASM remains within common hosting limits.

use std::fmt::Write as _;
use std::path::PathBuf;

#[path = "src/cjk_scripts.rs"]
mod cjk_scripts;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/easypdf-fonts");
    let mut src = String::from("pub static EASYPDF_FONTS: &[EasyPdfFont] = &[\n");
    match easy_pdf_fonts(&dir) {
        Ok(entries) => src.push_str(&entries),
        Err(e) => println!("cargo::error=invalid internal easyPDF font bundle at {}: {e}", dir.display()),
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("easypdf_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

fn easy_pdf_fonts(dir: &std::path::Path) -> Result<String, String> {
    let manifest = dir.join("fonts/manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let fields: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = fields.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        if cjk_scripts::is_barred_family(family) {
            return Err(format!("font family is forbidden by AGENTS.md: {family} {style}"));
        }
        let arabic = scripts.split(',').any(|s| s.trim() == "Arab");
        let telugu = scripts.split(',').any(|s| s.trim() == "Telu");
        let hebrew = scripts.split(',').any(|s| s.trim() == "Hebr");
        let ui_script = scripts.split(',').any(|s| matches!(s.trim(), "Thai" | "Deva" | "Hang" | "Laoo"));
        let web_face = (*family == "BIZ UDPGothic" && *style == "Regular") || arabic || telugu || hebrew || ui_script;
        if wasm && !web_face {
            continue;
        }
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        let cjk = scripts.split(',').any(cjk_scripts::is_cjk_script);
        let bytes = std::fs::metadata(&path).map_err(|e| format!("{file}: {e}"))?.len();
        if cjk && cjk_scripts::cjk_size_decision(family, bytes, wasm) == cjk_scripts::CjkSizeDecision::Skip {
            continue;
        }
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts.split(',').map(|s| format!("{:?}", s.trim())).collect();
        let _ = writeln!(
            out,
            "    EasyPdfFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}) }},",
            scripts.join(", "),
            path.display().to_string(),
        );
    }
    if out.is_empty() {
        return Err("manifest did not emit any usable font".into());
    }
    Ok(out)
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    #[test]
    fn manifest_requires_an_existing_font() {
        let root = std::env::temp_dir().join(format!("easypdf-font-manifest-{}", std::process::id()));
        std::fs::create_dir_all(root.join("fonts")).unwrap();
        std::fs::write(root.join("fonts/manifest.txt"), "Fixture | Regular | fonts/face.ttf | Latn\n").unwrap();
        assert!(easy_pdf_fonts(&root).is_err());
        std::fs::write(root.join("fonts/face.ttf"), b"synthetic manifest fixture").unwrap();
        assert!(easy_pdf_fonts(&root).unwrap().contains("EasyPdfFont {"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
