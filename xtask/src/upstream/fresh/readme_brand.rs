//! Replace the upstream marketing README with the reviewed easyPDF project README.

use std::path::Path;

use anyhow::{Result, bail};

use super::{atomic_write, read_bounded, sha256};

const LOCAL_I18N_GUIDE: &str = r#"Dalla cartella principale del progetto:

Per usare la lingua del sistema anche nelle compilazioni locali, prepara una volta
i cataloghi alla revisione fissata per questa release. Senza cataloghi, la build
di sviluppo mostra soltanto l'inglese.

```powershell
$catalog = Join-Path (Get-Location) 'target\craft-i18n'
if (-not (Test-Path $catalog)) { git clone https://github.com/storytold/craft-i18n.git $catalog }
git -C $catalog checkout --detach 575cfdef63227b14daf173b8cc87928bfdfa325e
$env:CRAFT_I18N_DIR = $catalog
$env:CRAFT_I18N_REQUIRED = '1'
```
"#;

pub(super) fn apply(root: &Path, stage: &Path, release: &str, backup_label: &str) -> Result<()> {
    if !matches!(release, "v0.18.2" | "v0.19.0") {
        return Ok(());
    }
    let owner = read_bounded(&root.join("README.md"))?;
    let upstream = read_bounded(&stage.join("README.md"))?;
    let upstream_hash = if release == "v0.19.0" {
        "00A62B720E023103B1A9404163AB78745F4416E9F6407EBBFC54B2D0218041A2"
    } else {
        "A79976F5B477A3A311ED226341EDBF42CDCFB61C021B6A40DA0BEA17F7AAF34C"
    };
    if !sha256(&owner).eq_ignore_ascii_case("2F00E2AE9135F9AAE477D460582765860914DD7D7C5D608B6CAD267A860868BC")
        || !sha256(&upstream).eq_ignore_ascii_case(upstream_hash)
    {
        bail!("README content changed; review before replay");
    }
    let text = String::from_utf8(owner)?;
    let branded = text
        .replace("assets/easyPDF-icona.png", "assets/app-icon/easypdf-1024.png")
        .replace("assets/easyPDF-logo-h.svg", "web-portal/public/assets/easyPDF-logo-h.svg");
    let guide_anchor = "Dalla cartella principale del progetto:\n";
    if branded.matches(guide_anchor).count() != 1 {
        bail!("reviewed easyPDF README has no unique local build guide anchor");
    }
    let branded = branded.replacen(guide_anchor, LOCAL_I18N_GUIDE, 1);
    if !stage.join("assets/app-icon/easypdf-1024.png").is_file()
        || !stage.join("web-portal/public/assets/easyPDF-logo-h.svg").is_file()
        || branded.contains("PdfCraft")
    {
        bail!("reviewed easyPDF README has missing assets or legacy branding");
    }
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-readme-before-apply/README.md"));
    if backup.exists() {
        bail!("README backup already exists: {}", backup.display());
    }
    atomic_write(&backup, &upstream)?;
    atomic_write(&stage.join("README.md"), branded.as_bytes())
}
