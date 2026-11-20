//! A standalone upstream editor ZIP would bypass easyPDF's account/organisation portal.
//! Keep the web build available through the existing authenticated Docker host instead.

use std::path::Path;

use anyhow::{Context, Result, bail};

use super::{atomic_write, read_bounded, sha256};

const INPUTS: [(&str, &str); 4] = [
    ("packaging/icons.sh", "17D109CC9616F17E70911A66FE6A1CF90CADA2139A991B542E2A95B48AA30D6C"),
    ("packaging/web/package.sh", "00FE458B4162BF055988221BC719C59334A96EFDE6B2206CB66CA805F70C8A57"),
    ("packaging/web/README.md", "A15E4B5B50D7F5C59EE2490EBC5654880FE9C6E399CCDF6154DC1CAFE317EF36"),
    ("packaging/macos/import-cert.sh", "B8B5634D8D3CBDACB9AABC573D3FCD36AAB63AD6D5FA3810395B51322DBC6541"),
];

const AUTHENTICATED_WEB_ONLY: &str = "#!/usr/bin/env bash
set -euo pipefail
echo 'error: standalone editor ZIP disabled: easyPDF web requires the authenticated web-portal host. Build the Docker stack with compose.yaml instead.' >&2
exit 2
";

const WEB_README: &str = "# Hosting easyPDF web

The editor is part of the authenticated easyPDF portal. Do not publish the
editor WASM and JavaScript as a standalone static site: that would bypass
account, organisation, cover-kit and document-sharing access controls.

For a local integration test, from the repository root run
`docker compose up --build -d`, then open `http://localhost:8282/` and sign in.
The editor is served at `/editor/` by the same-origin portal. The checked-in
`compose.yaml` contains development-only credentials and must not be deployed
unchanged to a public server. The standalone ZIP command deliberately fails
until a secure distributable for a configured portal is reviewed.
";

const COMPOSE_SHA256: &str = "73CACC988DBD81854C90B41203538EE84218136FDB9098E93D9F511CC17E7127";

pub(super) fn configure_staging_port(root: &Path, stage: &Path, release: &str, backup_label: &str) -> Result<()> {
    let relative = "compose.yaml";
    let path = stage.join(relative);
    let original = read_bounded(&path)?;
    if !sha256(&original).eq_ignore_ascii_case(COMPOSE_SHA256) {
        bail!("staging compose changed; review database, project name and port isolation");
    }
    let mut updated = String::from_utf8(original.clone()).context("staging compose is not UTF-8")?;
    let (project, port) = match release {
        "v0.18.1" => ("easypdf-0181-staging", "8383"),
        "v0.18.2" => ("easypdf-0182-staging", "8384"),
        "v0.19.0" => ("easypdf-0190-staging", "8385"),
        _ => bail!("staging compose isolation is not reviewed for {release}"),
    };
    for (old, new) in [
        ("name: easypdf-20", format!("name: {project}")),
        ("\"8282:80\"", format!("\"{port}:80\"")),
        ("APP_URL: http://localhost:8282", format!("APP_URL: http://localhost:{port}")),
    ] {
        if updated.matches(old).count() != 1 {
            bail!("staging compose isolation anchor changed: {old}");
        }
        updated = updated.replacen(old, &new, 1);
    }
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-compose-before-apply"));
    if backup.exists() {
        bail!("staging compose backup already exists: {}", backup.display());
    }
    atomic_write(&backup.join(relative), &original)?;
    atomic_write(&path, updated.as_bytes())?;
    Ok(())
}

pub(super) fn apply(root: &Path, stage: &Path, release: &str, backup_label: &str) -> Result<()> {
    if !matches!(release, "v0.18.0" | "v0.18.1" | "v0.18.2" | "v0.19.0") {
        bail!("web package adaptation is reviewed only for v0.18.0 and v0.18.1");
    }
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-web-package-before-apply"));
    if backup.exists() {
        bail!("web package backup already exists: {}", backup.display());
    }
    let mut originals = Vec::with_capacity(INPUTS.len());
    for (relative, expected) in INPUTS {
        let expected = if release == "v0.19.0" && relative == "packaging/web/README.md" {
            "2010D500ADA8D841BC80877533F7B784C22436A44E665FF26435D1D93B397FD3"
        } else {
            expected
        };
        let bytes = read_bounded(&stage.join(relative))?;
        if !sha256(&bytes).eq_ignore_ascii_case(expected) {
            bail!("web package input changed: {relative}; manual review required");
        }
        originals.push((relative, bytes));
    }
    let mut updated = Vec::with_capacity(originals.len());
    for (relative, bytes) in &originals {
        let original = std::str::from_utf8(bytes).with_context(|| format!("web package input is not UTF-8: {relative}"))?;
        let result = match *relative {
            "packaging/web/package.sh" => AUTHENTICATED_WEB_ONLY.to_owned(),
            "packaging/web/README.md" if release == "v0.18.1" => WEB_README.replace("localhost:8282", "localhost:8383"),
            "packaging/web/README.md" if release == "v0.18.2" => WEB_README.replace("localhost:8282", "localhost:8384"),
            "packaging/web/README.md" if release == "v0.19.0" => {
                let template = include_str!("web_page_host_v019.md");
                if !sha256(template.as_bytes()).eq_ignore_ascii_case("1ADBFC79746C33A9EAFF219909BFC69DD6BA5B39A53E05D10181DD406C3AEA4C") {
                    bail!("review required: easyPDF v0.19 web-host documentation changed");
                }
                template.to_owned()
            }
            "packaging/web/README.md" => WEB_README.to_owned(),
            _ => original.replace("ai.storyteller.pdfcraft", "it.twinba.easypdf").replace("PdfCraft", "easyPDF").replace("pdfcraft", "easypdf"),
        };
        if result.contains("PdfCraft") || result.contains("pdfcraft") || result.contains("storyteller") || result.contains("ArtCraft") {
            bail!("web package transformation left upstream identity: {relative}");
        }
        updated.push((relative, result));
    }
    for (relative, bytes) in &originals {
        atomic_write(&backup.join(relative), bytes)?;
    }
    for (relative, content) in updated {
        atomic_write(&stage.join(relative), content.as_bytes())?;
    }
    Ok(())
}
